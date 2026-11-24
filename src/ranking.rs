//! Leaderboard: o que o app envia para o Firebase e o que recebe de volta.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::sessoes::{self, Sessao};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ModoRanking {
    /// Uma linha por pessoa (com o jogo que ela mais jogou).
    Pessoas,
    /// Uma linha por pessoa + jogo.
    Pares,
    /// Quem mais jogou um jogo escolhido.
    PorJogo,
}

impl ModoRanking {
    pub const NOMES: [&'static str; 3] = ["Pessoas", "Pessoa + jogo", "Por jogo"];

    pub fn de(i: usize) -> Self {
        match i {
            0 => ModoRanking::Pessoas,
            1 => ModoRanking::Pares,
            _ => ModoRanking::PorJogo,
        }
    }

    pub fn indice(self) -> usize {
        self as usize
    }

    pub fn chave(self) -> &'static str {
        match self {
            ModoRanking::Pessoas => "pessoas",
            ModoRanking::Pares => "pares",
            ModoRanking::PorJogo => "jogo",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PeriodoRanking {
    Hoje,
    Semana,
    Mes,
    Tudo,
}

impl PeriodoRanking {
    pub const NOMES: [&'static str; 4] = ["Hoje", "Semana", "Mês", "Tudo"];
    pub const TODOS: [PeriodoRanking; 4] = [
        PeriodoRanking::Hoje,
        PeriodoRanking::Semana,
        PeriodoRanking::Mes,
        PeriodoRanking::Tudo,
    ];

    pub fn de(i: usize) -> Self {
        Self::TODOS[i.min(3)]
    }

    pub fn indice(self) -> usize {
        self as usize
    }

    pub fn nome(self) -> &'static str {
        Self::NOMES[self.indice()]
    }

    pub fn chave(self) -> &'static str {
        match self {
            PeriodoRanking::Hoje => "hoje",
            PeriodoRanking::Semana => "semana",
            PeriodoRanking::Mes => "mes",
            PeriodoRanking::Tudo => "tudo",
        }
    }
}

/// De onde vem o tempo de jogo.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlataformaRanking {
    Todas,
    Mobile,
    Steam,
    Psn,
}

impl PlataformaRanking {
    pub const NOMES: [&'static str; 4] = ["Todas", "Mobile", "Steam", "PSN"];
    pub const TODAS: [PlataformaRanking; 4] = [
        PlataformaRanking::Todas,
        PlataformaRanking::Mobile,
        PlataformaRanking::Steam,
        PlataformaRanking::Psn,
    ];

    pub fn de(i: usize) -> Self {
        Self::TODAS[i.min(3)]
    }

    /// Plataforma de um pacote: celular, Steam ou PSN.
    pub fn do_pacote(pacote: &str) -> Self {
        match crate::steam::Fonte::de_pacote(pacote) {
            Some(f) => Self::da_fonte(f),
            None => PlataformaRanking::Mobile,
        }
    }

    pub fn da_fonte(f: crate::steam::Fonte) -> Self {
        match f {
            crate::steam::Fonte::Steam => PlataformaRanking::Steam,
            crate::steam::Fonte::Psn => PlataformaRanking::Psn,
        }
    }

    pub fn indice(self) -> usize {
        self as usize
    }

    pub fn nome(self) -> &'static str {
        Self::NOMES[self.indice()]
    }

    pub fn chave(self) -> &'static str {
        match self {
            PlataformaRanking::Todas => "todas",
            PlataformaRanking::Mobile => "mobile",
            PlataformaRanking::Steam => "steam",
            PlataformaRanking::Psn => "psn",
        }
    }
}

/// Nome do campo no Firebase com o tempo de um período numa plataforma.
/// "Hoje" leva o dia junto ("hoje_2026-11-18_steam"), assim o ranking de hoje só pega quem jogou hoje
/// e não precisa de índice composto (é a mesma conta no MainActivity.java).
pub fn campo(periodo: PeriodoRanking, plataforma: PlataformaRanking, dia: &str) -> String {
    match periodo {
        PeriodoRanking::Hoje => format!("hoje_{dia}_{}", plataforma.chave()),
        p => format!("{}_{}", p.chave(), plataforma.chave()),
    }
}

/// Uma linha do Leaderboard.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ItemRanking {
    #[serde(default)]
    pub uid: String,
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub foto: String,
    #[serde(default)]
    pub pacote: String,
    #[serde(default)]
    pub jogo: String,
    /// Chave do jogo pelo nome (ver `chave_jogo`).
    #[serde(default)]
    pub chave: String,
    #[serde(default)]
    pub ms: i64,
    #[serde(default)]
    pub atualizado_ms: i64,
    /// Onde o jogo existe: "mobile", "steam", "psn" (um ou mais).
    #[serde(default)]
    pub plats: Vec<String>,
    /// Moldura da foto ("ruby", "diamante", "bronze" ou vazio).
    #[serde(default)]
    pub moldura: String,
}

/// Chave de um jogo pelo nome, para juntar o mesmo jogo instalado de lojas diferentes
/// (ex.: Roblox da Play Store e da Galaxy Store). "Roblox" -> "roblox".
pub fn chave_jogo(nome: &str, pacote: &str) -> String {
    let k: String = nome
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect();
    if k.is_empty() {
        pacote
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    } else {
        k
    }
}

pub fn ler_itens(json: &str) -> Result<Vec<ItemRanking>, String> {
    serde_json::from_str(json).map_err(|e| format!("Resposta estranha do servidor: {e}"))
}

#[derive(Serialize, PartialEq, Clone)]
struct Top {
    pacote: String,
    nome: String,
    chave: String,
    plats: Vec<&'static str>,
}

#[derive(Serialize, PartialEq)]
struct JogoPub {
    pacote: String,
    nome: String,
    chave: String,
    plats: Vec<&'static str>,
    // formato antigo (versões até a 2.0.1 leem estes)
    hoje_ms: i64,
    semana_ms: i64,
    tudo_ms: i64,
    /// campo (ver `campo`) -> ms
    r: BTreeMap<String, i64>,
}

#[derive(Serialize, PartialEq)]
struct Publicacao {
    dia: String,
    hoje_ms: i64,
    semana_ms: i64,
    tudo_ms: i64,
    top_hoje: Option<Top>,
    top_semana: Option<Top>,
    top_tudo: Option<Top>,
    r: BTreeMap<String, i64>,
    top: BTreeMap<String, Top>,
    jogos: Vec<JogoPub>,
}

/// Começo de cada período (ms) e o "agora".
pub struct Intervalos {
    pub agora: i64,
    pub hoje: i64,
    pub semana: i64,
    pub mes: i64,
}

/// Uma sessão já com o pacote "principal" do jogo e a plataforma de onde veio
/// (Mobile, Steam ou Psn; nunca Todas).
pub struct SessaoPub {
    pub sessao: Sessao,
    pub plataforma: PlataformaRanking,
}

/// Monta o JSON com o tempo da pessoa (só jogos), por período e por plataforma.
/// - `bases`: tempo antigo da Steam/PSN (pacote principal, ms, plataforma), que só conta em "Tudo";
/// - `nome`: nome de cada pacote; `plats`: onde o jogo existe ("mobile", "steam", "psn").
pub fn montar_publicacao(
    sessoes: &[SessaoPub],
    bases: &[(String, i64, PlataformaRanking)],
    nome: impl Fn(&str) -> String,
    plats: impl Fn(&str) -> Vec<&'static str>,
    iv: &Intervalos,
    dia: &str,
) -> String {
    // pacote -> [período][plataforma] (plataforma: índice de PlataformaRanking; 0 = todas)
    let mut por_jogo: HashMap<&str, [[i64; 4]; 4]> = HashMap::new();
    for sp in sessoes {
        let s = &sp.sessao;
        let e = por_jogo.entry(s.pacote.as_str()).or_insert([[0; 4]; 4]);
        let t = [
            sessoes::tempo_no_intervalo(s, iv.hoje, iv.agora),
            sessoes::tempo_no_intervalo(s, iv.semana, iv.agora),
            sessoes::tempo_no_intervalo(s, iv.mes, iv.agora),
            s.duracao(),
        ];
        let pl = sp.plataforma.indice().max(1);
        for (i, v) in t.into_iter().enumerate() {
            e[i][0] += v;
            e[i][pl] += v;
        }
    }
    for (p, ms, pl) in bases {
        if *ms > 0 {
            let e = por_jogo.entry(p.as_str()).or_insert([[0; 4]; 4]);
            e[3][0] += ms;
            e[3][pl.indice().max(1)] += ms;
        }
    }

    let campos: Vec<Vec<String>> = PeriodoRanking::TODOS
        .iter()
        .map(|per| PlataformaRanking::TODAS.iter().map(|pl| campo(*per, *pl, dia)).collect())
        .collect();

    let mut jogos: Vec<JogoPub> = por_jogo
        .into_iter()
        .map(|(p, t)| {
            let n = nome(p);
            let mut r = BTreeMap::new();
            for (per, linha) in t.iter().enumerate() {
                for (pl, v) in linha.iter().enumerate() {
                    if *v > 0 {
                        r.insert(campos[per][pl].clone(), *v);
                    }
                }
            }
            JogoPub {
                chave: chave_jogo(&n, p),
                plats: plats(p),
                pacote: p.to_string(),
                nome: n,
                hoje_ms: t[0][0],
                semana_ms: t[1][0],
                tudo_ms: t[3][0],
                r,
            }
        })
        .filter(|j| !j.r.is_empty())
        .collect();
    jogos.sort_by(|a, b| b.tudo_ms.cmp(&a.tudo_ms).then(a.pacote.cmp(&b.pacote)));

    // totais da pessoa e o jogo que ela mais jogou em cada campo
    let mut r: BTreeMap<String, i64> = BTreeMap::new();
    let mut top: BTreeMap<String, (i64, Top)> = BTreeMap::new();
    for j in &jogos {
        for (k, v) in &j.r {
            *r.entry(k.clone()).or_insert(0) += v;
            let melhor = top.get(k).map_or(true, |(m, _)| v > m);
            if melhor {
                top.insert(
                    k.clone(),
                    (
                        *v,
                        Top {
                            pacote: j.pacote.clone(),
                            nome: j.nome.clone(),
                            chave: j.chave.clone(),
                            plats: j.plats.clone(),
                        },
                    ),
                );
            }
        }
    }
    let top: BTreeMap<String, Top> = top.into_iter().map(|(k, (_, t))| (k, t)).collect();

    // Um documento por jogo no Firebase: manda os jogados no último mês + os 30 com mais tempo.
    let mes_todas = &campos[2][0];
    let mut i = 0;
    jogos.retain(|j| {
        i += 1;
        i <= 30 || j.r.contains_key(mes_todas)
    });
    jogos.truncate(80);

    let top_antigo = |per: usize| top.get(&campos[per][0]).cloned();
    let p = Publicacao {
        dia: dia.to_string(),
        hoje_ms: r.get(&campos[0][0]).copied().unwrap_or(0),
        semana_ms: r.get(&campos[1][0]).copied().unwrap_or(0),
        tudo_ms: r.get(&campos[3][0]).copied().unwrap_or(0),
        top_hoje: top_antigo(0),
        top_semana: top_antigo(1),
        top_tudo: top_antigo(3),
        r,
        top,
        jogos,
    };
    serde_json::to_string(&p).unwrap_or_default()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn publicacao_soma_e_escolhe_o_top() {
        let sp = |p: &str, inicio, fim, plataforma| SessaoPub {
            sessao: Sessao {
                pacote: p.into(),
                inicio,
                fim,
            },
            plataforma,
        };
        use PlataformaRanking::{Mobile, Psn, Steam};
        let s = vec![
            sp("a", 0, 1000, Mobile),
            sp("b", 5000, 9000, Mobile),
            sp("b", 9000, 9500, Steam),
            sp("d", 9500, 9600, Psn),
        ];
        let bases = vec![("c".to_string(), 7000, Steam)];
        let iv = Intervalos {
            agora: 10_000,
            hoje: 4000,
            semana: 0,
            mes: 0,
        };
        let json = montar_publicacao(&s, &bases, |p| p.to_uppercase(), |_| vec!["mobile"], &iv, "2026-11-17");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["tudo_ms"], 12600);
        assert_eq!(v["hoje_ms"], 4600);
        assert_eq!(v["r"]["hoje_2026-11-17_psn"], 100);
        assert!(v["r"].get("hoje_2026-11-17_psn").is_some());
        assert_eq!(v["top_tudo"]["pacote"], "c");
        assert_eq!(v["top_hoje"]["nome"], "B");
        assert_eq!(v["r"]["hoje_2026-11-17_steam"], 500);
        assert_eq!(v["r"]["hoje_2026-11-17_mobile"], 4000);
        assert_eq!(v["r"]["tudo_steam"], 7500);
        assert_eq!(v["top"]["tudo_mobile"]["pacote"], "b");
        assert_eq!(v["top"]["tudo_steam"]["pacote"], "c");
        assert!(v["r"].get("mes_steam").is_some());
        assert_eq!(v["jogos"][0]["chave"], "c");
        assert!(v["jogos"][0]["r"].get("hoje_2026-11-17_todas").is_none());
    }

    #[test]
    fn campos() {
        assert_eq!(
            campo(PeriodoRanking::Hoje, PlataformaRanking::Steam, "2026-11-18"),
            "hoje_2026-11-18_steam"
        );
        assert_eq!(campo(PeriodoRanking::Mes, PlataformaRanking::Todas, "x"), "mes_todas");
    }

    #[test]
    fn chave_pelo_nome() {
        assert_eq!(chave_jogo("Roblox", "com.roblox.client"), "roblox");
        assert_eq!(chave_jogo(" ROBLOX ", "x"), "roblox");
        assert_eq!(chave_jogo("Pokémon GO", "x"), "pokémongo");
        assert_eq!(chave_jogo("", "com.a.b"), "com_a_b");
    }

    #[test]
    fn le_itens() {
        let v = ler_itens(r#"[{"uid":"x","nome":"Riko","ms":60000}]"#).unwrap();
        assert_eq!(v[0].nome, "Riko");
        assert_eq!(v[0].pacote, "");
    }
}
