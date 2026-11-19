//! Histórico por dia, para o Placar e os Eventos.
//!
//! Cada pessoa publica, uma vez a cada poucos minutos, quanto jogou em cada dia (até ~400 dias),
//! separado por plataforma e por jogo. Com isso dá para saber quem ganhou qualquer dia, semana
//! (segunda a domingo) ou mês, e calcular os eventos, sem precisar de servidor.

use std::collections::{BTreeMap, HashMap};
use std::ops::Bound;

use chrono::{Datelike, Days, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};

use crate::ranking::{chave_jogo, ItemRanking, ModoRanking, PlataformaRanking, SessaoPub};
use crate::sessoes;

/// Até quantos dias para trás o histórico é publicado.
pub const DIAS_MAX: u64 = 400;

const MESES: [&str; 12] = [
    "janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro",
    "novembro", "dezembro",
];
const MESES_CURTOS: [&str; 12] = [
    "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
];
const SEMANA: [&str; 7] = ["Segunda", "Terça", "Quarta", "Quinta", "Sexta", "Sábado", "Domingo"];

// ---------------------------------------------------------------------------
// Datas
// ---------------------------------------------------------------------------

pub fn hoje() -> NaiveDate {
    Local::now().date_naive()
}

pub fn fmt_dia(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub fn ler_dia(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

pub fn mais_dias(d: NaiveDate, n: i64) -> NaiveDate {
    let r = if n >= 0 {
        d.checked_add_days(Days::new(n as u64))
    } else {
        d.checked_sub_days(Days::new((-n) as u64))
    };
    r.unwrap_or(d)
}

/// ms do começo do dia (no fuso do celular).
pub fn inicio_ms(d: NaiveDate) -> i64 {
    d.and_hms_opt(0, 0, 0)
        .and_then(|t| t.and_local_timezone(Local).earliest())
        .map(|t| t.timestamp_millis())
        .unwrap_or(0)
}

fn dia_do_ms(ms: i64) -> NaiveDate {
    Local
        .timestamp_millis_opt(ms)
        .earliest()
        .map(|t| t.date_naive())
        .unwrap_or_else(hoje)
}

/// "18 nov 2026"
pub fn fmt_data_curta(d: NaiveDate) -> String {
    format!("{} {} {}", d.day(), MESES_CURTOS[d.month0() as usize], d.year())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TipoPeriodo {
    Dia,
    Semana,
    Mes,
}

impl TipoPeriodo {
    /// Primeiro e último dia (inclusive) do período que contém `d`. A semana vai de segunda a domingo.
    pub fn intervalo(self, d: NaiveDate) -> (NaiveDate, NaiveDate) {
        match self {
            TipoPeriodo::Dia => (d, d),
            TipoPeriodo::Semana => {
                let ini = mais_dias(d, -(d.weekday().num_days_from_monday() as i64));
                (ini, mais_dias(ini, 6))
            }
            TipoPeriodo::Mes => {
                let ini = d.with_day(1).unwrap_or(d);
                let prox = if ini.month() == 12 {
                    NaiveDate::from_ymd_opt(ini.year() + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(ini.year(), ini.month() + 1, 1)
                }
                .unwrap_or(ini);
                (ini, mais_dias(prox, -1))
            }
        }
    }

    /// O período logo antes do que começa em `ini`.
    pub fn anterior(self, ini: NaiveDate) -> (NaiveDate, NaiveDate) {
        self.intervalo(mais_dias(ini, -1))
    }

    /// "Terça, 18 nov 2026" · "17 a 23 nov 2026" · "Novembro de 2026"
    pub fn rotulo(self, ini: NaiveDate, fim: NaiveDate) -> String {
        match self {
            TipoPeriodo::Dia => format!(
                "{}, {}",
                SEMANA[ini.weekday().num_days_from_monday() as usize],
                fmt_data_curta(ini)
            ),
            TipoPeriodo::Semana => rotulo_intervalo(ini, fim),
            TipoPeriodo::Mes => {
                let m = MESES[ini.month0() as usize];
                let mut c = m.chars();
                let m = match c.next() {
                    Some(p) => p.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                };
                format!("{m} de {}", ini.year())
            }
        }
    }
}

/// "17 a 23 nov 2026", "28 out a 3 nov 2026", "28 dez 2025 a 3 jan 2026"
pub fn rotulo_intervalo(ini: NaiveDate, fim: NaiveDate) -> String {
    if ini == fim {
        fmt_data_curta(ini)
    } else if ini.year() != fim.year() {
        format!("{} a {}", fmt_data_curta(ini), fmt_data_curta(fim))
    } else if ini.month() != fim.month() {
        format!(
            "{} {} a {}",
            ini.day(),
            MESES_CURTOS[ini.month0() as usize],
            fmt_data_curta(fim)
        )
    } else {
        format!("{} a {}", ini.day(), fmt_data_curta(fim))
    }
}

// ---------------------------------------------------------------------------
// Publicar o seu histórico
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct InfoPub {
    nome: String,
    pacote: String,
    plats: Vec<&'static str>,
}

#[derive(Serialize)]
struct HistPub {
    /// dia -> plataforma ("todas", "mobile", "steam", "psn") -> ms
    dias: BTreeMap<String, BTreeMap<&'static str, i64>>,
    /// dia -> chave do jogo -> plataforma -> ms
    jogos: BTreeMap<String, BTreeMap<String, BTreeMap<&'static str, i64>>>,
    /// chave do jogo -> nome, pacote e onde ele existe
    info: BTreeMap<String, InfoPub>,
}

/// Monta o JSON do seu histórico (só jogos, com os repetidos juntados; `sessoes` já vem assim).
pub fn montar_historico(
    sessoes: &[SessaoPub],
    nome: impl Fn(&str) -> String,
    plats: impl Fn(&str) -> Vec<&'static str>,
    agora: i64,
) -> String {
    let hoje = dia_do_ms(agora);
    let limite = mais_dias(hoje, -(DIAS_MAX as i64));
    let limite_ms = inicio_ms(limite);
    let mut h = HistPub {
        dias: BTreeMap::new(),
        jogos: BTreeMap::new(),
        info: BTreeMap::new(),
    };
    for sp in sessoes {
        let s = &sp.sessao;
        if s.fim <= limite_ms {
            continue;
        }
        let pl = sp.plataforma.chave();
        let n = nome(&s.pacote);
        let ch = chave_jogo(&n, &s.pacote);
        // divide a sessão pelos dias que ela passou (ex.: das 23h à 1h)
        let mut d = dia_do_ms(s.inicio.max(limite_ms));
        let mut contou = false;
        while d <= hoje {
            let ini = inicio_ms(d);
            if ini >= s.fim {
                break;
            }
            let t = sessoes::tempo_no_intervalo(s, ini, inicio_ms(mais_dias(d, 1)));
            if t > 0 {
                let k = fmt_dia(d);
                let dia = h.dias.entry(k.clone()).or_default();
                *dia.entry("todas").or_insert(0) += t;
                *dia.entry(pl).or_insert(0) += t;
                let jogo = h.jogos.entry(k).or_default().entry(ch.clone()).or_default();
                *jogo.entry("todas").or_insert(0) += t;
                *jogo.entry(pl).or_insert(0) += t;
                contou = true;
            }
            d = mais_dias(d, 1);
        }
        if contou && !h.info.contains_key(&ch) {
            h.info.insert(
                ch,
                InfoPub {
                    nome: n,
                    pacote: s.pacote.clone(),
                    plats: plats(&s.pacote),
                },
            );
        }
    }
    serde_json::to_string(&h).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Ler o histórico de todo mundo e montar rankings
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct InfoJogo {
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub pacote: String,
    #[serde(default)]
    pub plats: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct HistoricoPessoa {
    #[serde(default)]
    pub uid: String,
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub foto: String,
    #[serde(default)]
    pub atualizado_ms: i64,
    #[serde(default)]
    pub dias: BTreeMap<String, BTreeMap<String, i64>>,
    #[serde(default)]
    pub jogos: BTreeMap<String, BTreeMap<String, BTreeMap<String, i64>>>,
    #[serde(default)]
    pub info: BTreeMap<String, InfoJogo>,
}

pub fn ler_historicos(json: &str) -> Result<Vec<HistoricoPessoa>, String> {
    serde_json::from_str(json).map_err(|e| format!("Resposta estranha do servidor: {e}"))
}

/// O dia mais antigo com tempo de jogo em algum histórico.
pub fn primeiro_dia(hist: &[HistoricoPessoa]) -> Option<NaiveDate> {
    hist.iter()
        .filter_map(|h| h.dias.keys().next())
        .min()
        .and_then(|k| ler_dia(k))
}

/// Todos os jogos que aparecem no histórico de alguém: (chave, pacote, nome).
pub fn jogos_conhecidos(hist: &[HistoricoPessoa]) -> Vec<(String, String, String)> {
    let mut m: BTreeMap<String, (String, String)> = BTreeMap::new();
    for h in hist {
        for (k, i) in &h.info {
            m.entry(k.clone()).or_insert((i.pacote.clone(), i.nome.clone()));
        }
    }
    let mut v: Vec<(String, String, String)> = m.into_iter().map(|(k, (p, n))| (k, p, n)).collect();
    v.sort_by(|a, b| a.2.to_lowercase().cmp(&b.2.to_lowercase()));
    v
}

fn item(h: &HistoricoPessoa, chave: Option<&str>, ms: i64) -> ItemRanking {
    let info = chave.and_then(|c| h.info.get(c)).cloned().unwrap_or_default();
    ItemRanking {
        uid: h.uid.clone(),
        nome: h.nome.clone(),
        foto: h.foto.clone(),
        pacote: info.pacote,
        jogo: info.nome,
        chave: chave.unwrap_or("").to_string(),
        ms,
        atualizado_ms: h.atualizado_ms,
        plats: info.plats,
    }
}

/// Ranking entre os dias `ini` e `fim` (inclusive, "AAAA-MM-DD").
/// `jogo` é a chave do jogo, usada no modo "Por jogo".
pub fn ranking(
    hist: &[HistoricoPessoa],
    ini: &str,
    fim: &str,
    modo: ModoRanking,
    plataforma: PlataformaRanking,
    jogo: Option<&str>,
) -> Vec<ItemRanking> {
    if ini > fim {
        return Vec::new();
    }
    let pl = plataforma.chave();
    let faixa = (Bound::Included(ini), Bound::Included(fim));
    let mut v = Vec::new();
    for h in hist {
        // chave do jogo -> ms no período
        let mut por_jogo: HashMap<&str, i64> = HashMap::new();
        for (_, js) in h.jogos.range::<str, _>(faixa) {
            for (ch, m) in js {
                let t = m.get(pl).copied().unwrap_or(0);
                if t > 0 {
                    *por_jogo.entry(ch.as_str()).or_insert(0) += t;
                }
            }
        }
        match modo {
            ModoRanking::Pessoas => {
                let ms: i64 = h
                    .dias
                    .range::<str, _>(faixa)
                    .map(|(_, m)| m.get(pl).copied().unwrap_or(0))
                    .sum();
                if ms > 0 {
                    let top = por_jogo
                        .iter()
                        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
                        .map(|(k, _)| *k);
                    v.push(item(h, top, ms));
                }
            }
            ModoRanking::Pares => {
                for (ch, ms) in &por_jogo {
                    v.push(item(h, Some(*ch), *ms));
                }
            }
            ModoRanking::PorJogo => {
                if let Some(j) = jogo {
                    if let Some(ms) = por_jogo.get(j) {
                        v.push(item(h, Some(j), *ms));
                    }
                }
            }
        }
    }
    v.sort_by(|a, b| {
        b.ms.cmp(&a.ms)
            .then(a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
            .then(a.chave.cmp(&b.chave))
    });
    v
}

// ---------------------------------------------------------------------------
// Eventos
// ---------------------------------------------------------------------------

fn tres() -> i64 {
    3
}

/// Um evento criado pelo criador do app (coleção abysshub_eventos).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Evento {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub descricao: String,
    #[serde(default)]
    pub recompensas: String,
    /// "dia", "semana", "mes", "periodo" ou "tudo"
    #[serde(default)]
    pub tipo: String,
    /// primeiro dia, "AAAA-MM-DD"
    #[serde(default)]
    pub inicio: String,
    /// último dia (inclusive), "AAAA-MM-DD"; vazio = sem fim (só no "tudo")
    #[serde(default)]
    pub fim: String,
    #[serde(default)]
    pub inicio_ms: i64,
    /// fim do último dia (exclusivo); 0 = sem fim
    #[serde(default)]
    pub fim_ms: i64,
    /// "todas", "mobile", "steam" ou "psn"
    #[serde(default)]
    pub plataforma: String,
    /// vazio = tempo total; senão, a chave do jogo
    #[serde(default)]
    pub jogo_chave: String,
    #[serde(default)]
    pub jogo_nome: String,
    #[serde(default = "tres")]
    pub vencedores: i64,
    #[serde(default)]
    pub criado_ms: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Situacao {
    Futuro,
    Andamento,
    Terminado,
}

impl Evento {
    pub fn situacao(&self, agora: i64) -> Situacao {
        if agora < self.inicio_ms {
            Situacao::Futuro
        } else if self.fim_ms == 0 || agora < self.fim_ms {
            Situacao::Andamento
        } else {
            Situacao::Terminado
        }
    }

    pub fn plataforma(&self) -> PlataformaRanking {
        match self.plataforma.as_str() {
            "mobile" => PlataformaRanking::Mobile,
            "steam" => PlataformaRanking::Steam,
            "psn" => PlataformaRanking::Psn,
            _ => PlataformaRanking::Todas,
        }
    }

    pub fn qtd_vencedores(&self) -> usize {
        self.vencedores.clamp(1, 10) as usize
    }

    /// Dias que contam: ("AAAA-MM-DD", "AAAA-MM-DD"), sem passar de hoje.
    pub fn dias(&self, hoje: NaiveDate) -> (String, String) {
        let ini = if self.tipo == "tudo" {
            "0000-01-01".to_string()
        } else {
            self.inicio.clone()
        };
        let h = fmt_dia(hoje);
        let fim = if self.fim.is_empty() || self.fim > h { h } else { self.fim.clone() };
        (ini, fim)
    }

    /// Quem está na frente (ou ganhou).
    pub fn ranking(&self, hist: &[HistoricoPessoa], hoje: NaiveDate) -> Vec<ItemRanking> {
        let (ini, fim) = self.dias(hoje);
        let (modo, jogo) = if self.jogo_chave.is_empty() {
            (ModoRanking::Pessoas, None)
        } else {
            (ModoRanking::PorJogo, Some(self.jogo_chave.as_str()))
        };
        ranking(hist, &ini, &fim, modo, self.plataforma(), jogo)
    }

    /// "Semana de 17 a 23 nov 2026", "Tempo total até 31 dez 2026"...
    pub fn rotulo_periodo(&self) -> String {
        let ini = ler_dia(&self.inicio);
        let fim = ler_dia(&self.fim);
        match (self.tipo.as_str(), ini, fim) {
            ("dia", Some(i), _) => TipoPeriodo::Dia.rotulo(i, i),
            ("semana", Some(i), Some(f)) => format!("Semana de {}", rotulo_intervalo(i, f)),
            ("mes", Some(i), Some(f)) => TipoPeriodo::Mes.rotulo(i, f),
            ("periodo", Some(i), Some(f)) => format!("De {}", rotulo_intervalo(i, f)).replacen(" a ", " até ", 1),
            ("tudo", _, Some(f)) => format!("Tempo total até {}", fmt_data_curta(f)),
            ("tudo", _, None) => "Tempo total (sem data para acabar)".to_string(),
            _ => String::new(),
        }
    }
}

pub fn ler_eventos(json: &str) -> Result<Vec<Evento>, String> {
    serde_json::from_str(json).map_err(|e| format!("Resposta estranha do servidor: {e}"))
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::sessoes::Sessao;

    fn d(s: &str) -> NaiveDate {
        ler_dia(s).unwrap()
    }

    #[test]
    fn semana_de_segunda_a_domingo() {
        // 19/11/2026 é uma quinta
        let (i, f) = TipoPeriodo::Semana.intervalo(d("2026-11-19"));
        assert_eq!((fmt_dia(i), fmt_dia(f)), ("2026-11-16".into(), "2026-11-22".into()));
        let (i, f) = TipoPeriodo::Semana.anterior(i);
        assert_eq!((fmt_dia(i), fmt_dia(f)), ("2026-11-09".into(), "2026-11-15".into()));
        let (i, f) = TipoPeriodo::Mes.intervalo(d("2026-12-10"));
        assert_eq!((fmt_dia(i), fmt_dia(f)), ("2026-12-01".into(), "2026-12-31".into()));
        assert_eq!(TipoPeriodo::Mes.rotulo(i, f), "Dezembro de 2026");
        assert_eq!(rotulo_intervalo(d("2026-10-28"), d("2026-11-03")), "28 out a 3 nov 2026");
    }

    #[test]
    fn historico_divide_por_dia_e_ranking() {
        let meia_noite = inicio_ms(d("2026-11-19"));
        let s = vec![SessaoPub {
            sessao: Sessao {
                pacote: "com.jogo".into(),
                inicio: meia_noite - 3_600_000,
                fim: meia_noite + 1_800_000,
            },
            plataforma: PlataformaRanking::Mobile,
        }];
        let json = montar_historico(&s, |_| "Jogo X".into(), |_| vec!["mobile"], meia_noite + 7_200_000);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["dias"]["2026-11-18"]["todas"], 3_600_000);
        assert_eq!(v["dias"]["2026-11-19"]["mobile"], 1_800_000);
        assert_eq!(v["jogos"]["2026-11-19"]["jogox"]["todas"], 1_800_000);

        let mut h: HistoricoPessoa = serde_json::from_str(&json).unwrap();
        h.uid = "a".into();
        h.nome = "Ana".into();
        let r = ranking(&[h.clone()], "2026-11-16", "2026-11-22", ModoRanking::Pessoas, PlataformaRanking::Todas, None);
        assert_eq!(r[0].ms, 5_400_000);
        assert_eq!(r[0].jogo, "Jogo X");
        let r = ranking(&[h.clone()], "2026-11-19", "2026-11-19", ModoRanking::PorJogo, PlataformaRanking::Mobile, Some("jogox"));
        assert_eq!(r[0].ms, 1_800_000);
        assert!(ranking(&[h], "2026-11-19", "2026-11-19", ModoRanking::Pessoas, PlataformaRanking::Steam, None).is_empty());
    }

    #[test]
    fn evento_situacao() {
        let e = Evento {
            inicio_ms: 100,
            fim_ms: 200,
            ..Default::default()
        };
        assert_eq!(e.situacao(50), Situacao::Futuro);
        assert_eq!(e.situacao(150), Situacao::Andamento);
        assert_eq!(e.situacao(250), Situacao::Terminado);
    }
}
