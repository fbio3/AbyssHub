//! Jogos de outras plataformas: Steam (PC) e PlayStation (PS4/PS5).
//!
//! A Steam e a PSN só informam o tempo TOTAL de cada jogo (em minutos), não as sessões. Então:
//! - no dia em que a pessoa conecta, o total de cada jogo vira uma "base" que só conta em "Tudo";
//! - a cada atualização, o que cresceu desde a última vez vira uma sessão (com o fim no horário
//!   em que a Steam diz que o jogo foi jogado por último). É isso que conta em Hoje/Semana/Mês.
//!
//! Os jogos usam o pacote "steam:{appid}" ou "psn:{titleId}".

use serde::{Deserialize, Serialize};

use crate::dados::Dados;
use crate::sessoes::{self, Sessao};

/// De onde vêm os jogos que não são do celular.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Fonte {
    Steam = 0,
    Psn = 1,
}

impl Fonte {
    pub const TODAS: [Fonte; 2] = [Fonte::Steam, Fonte::Psn];

    pub fn prefixo(self) -> &'static str {
        match self {
            Fonte::Steam => "steam:",
            Fonte::Psn => "psn:",
        }
    }

    /// Mesmo nome usado no Leaderboard e no MainActivity.java ("steamJogos", "psnJogos"...).
    pub fn chave(self) -> &'static str {
        match self {
            Fonte::Steam => "steam",
            Fonte::Psn => "psn",
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Fonte::Steam => "Steam",
            Fonte::Psn => "PlayStation",
        }
    }

    pub fn pacote(self, id: &str) -> String {
        format!("{}{id}", self.prefixo())
    }

    /// De qual plataforma é o pacote (None = jogo do celular).
    pub fn de_pacote(pacote: &str) -> Option<Fonte> {
        Self::TODAS.into_iter().find(|f| pacote.starts_with(f.prefixo()))
    }

    /// O id do jogo na plataforma ("steam:730" -> "730").
    pub fn id_de(self, pacote: &str) -> Option<&str> {
        pacote.strip_prefix(self.prefixo())
    }

    pub fn dados(self, d: &Dados) -> &SteamDados {
        match self {
            Fonte::Steam => &d.steam,
            Fonte::Psn => &d.psn,
        }
    }

    pub fn dados_mut(self, d: &mut Dados) -> &mut SteamDados {
        match self {
            Fonte::Steam => &mut d.steam,
            Fonte::Psn => &mut d.psn,
        }
    }
}

/// O pacote é de um jogo da Steam ou da PSN.
pub fn eh_externo(pacote: &str) -> bool {
    Fonte::de_pacote(pacote).is_some()
}

/// O que o app guarda de cada jogo da Steam.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SteamJogo {
    #[serde(default)]
    pub nome: String,
    /// Tempo que o jogo já tinha quando a Steam foi conectada (só conta em "Tudo").
    #[serde(default)]
    pub base_ms: i64,
    /// Total (minutos) visto na última atualização.
    #[serde(default)]
    pub visto_min: i64,
    /// Quando foi jogado por último (ms), segundo a Steam.
    #[serde(default)]
    pub ultima_ms: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SteamDados {
    /// SteamID64 da conta conectada.
    #[serde(default)]
    pub id: String,
    /// Já fez a primeira leitura (a que vira "base").
    #[serde(default)]
    pub iniciado: bool,
    /// Quando foi a última atualização (ms).
    #[serde(default)]
    pub ultima_sync: i64,
    /// appid -> jogo
    #[serde(default)]
    pub jogos: std::collections::BTreeMap<String, SteamJogo>,
}

/// Um jogo como vem do MainActivity.steamJogos().
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct JogoApi {
    #[serde(deserialize_with = "texto_ou_numero")]
    pub appid: String,
    #[serde(default)]
    pub nome: String,
    /// Total jogado, em minutos.
    #[serde(default)]
    pub min: i64,
    /// Última vez que jogou (segundos desde 1970, 0 = não sei).
    #[serde(default)]
    pub ultima: i64,
}

#[derive(Deserialize)]
struct RespostaApi {
    #[serde(default)]
    id: String,
    #[serde(default)]
    jogos: Vec<JogoApi>,
}

fn texto_ou_numero<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    })
}

/// Lê o JSON de steamJogos(): {"id": "...", "jogos": [...]}.
pub fn ler(json: &str) -> Result<(String, Vec<JogoApi>), String> {
    let r: RespostaApi = serde_json::from_str(json).map_err(|e| format!("Resposta estranha da Steam: {e}"))?;
    Ok((r.id, r.jogos))
}

/// Tira do app tudo o que veio dessa plataforma.
pub fn remover(dados: &mut Dados, fonte: Fonte) {
    let p = fonte.prefixo();
    dados.sessoes.retain(|s| !s.pacote.starts_with(p));
    dados.nomes.retain(|k, _| !k.starts_with(p));
    dados.auto_jogo.retain(|k| !k.starts_with(p));
    dados.marcados_jogo.retain(|k| !k.starts_with(p));
    dados.desmarcados_jogo.retain(|k| !k.starts_with(p));
    *fonte.dados_mut(dados) = SteamDados::default();
}

/// Junta uma leitura da Steam/PSN aos dados. Devolve quantas sessões novas foram criadas.
pub fn aplicar(dados: &mut Dados, fonte: Fonte, id: &str, jogos: &[JogoApi], agora: i64) -> usize {
    {
        let d = fonte.dados(dados);
        if !d.id.is_empty() && d.id != id {
            // outra conta: começa do zero
            remover(dados, fonte);
        }
    }
    fonte.dados_mut(dados).id = id.to_string();
    let primeira = !fonte.dados(dados).iniciado;
    let ultima_sync = fonte.dados(dados).ultima_sync;
    let mut novas = 0;
    for j in jogos {
        if j.min <= 0 || j.appid.is_empty() {
            continue;
        }
        let p = fonte.pacote(&j.appid);
        if !j.nome.is_empty() {
            dados.nomes.insert(p.clone(), j.nome.clone());
        }
        dados.auto_jogo.insert(p.clone());
        let ultima_ms = j.ultima.max(0) * 1000;
        let e = fonte.dados_mut(dados).jogos.entry(j.appid.clone()).or_insert_with(|| {
            if primeira {
                // o que já tinha antes de conectar: só conta em "Tudo"
                SteamJogo {
                    base_ms: j.min * 60_000,
                    visto_min: j.min,
                    ..Default::default()
                }
            } else {
                // jogo novo depois de conectar: todo o tempo dele é novo
                SteamJogo::default()
            }
        });
        if !j.nome.is_empty() {
            e.nome = j.nome.clone();
        }
        let delta = j.min - e.visto_min;
        e.visto_min = e.visto_min.max(j.min);
        e.ultima_ms = e.ultima_ms.max(ultima_ms);
        if delta > 0 {
            let fim = if ultima_ms > ultima_sync && ultima_ms <= agora {
                ultima_ms
            } else {
                agora
            };
            sessoes::mesclar(
                &mut dados.sessoes,
                Sessao {
                    pacote: p,
                    inicio: fim - delta * 60_000,
                    fim,
                },
            );
            novas += 1;
        }
    }
    let d = fonte.dados_mut(dados);
    d.iniciado = true;
    d.ultima_sync = agora;
    novas
}

#[cfg(test)]
mod testes {
    use super::*;

    fn j(appid: &str, min: i64, ultima: i64) -> JogoApi {
        JogoApi {
            appid: appid.into(),
            nome: format!("Jogo {appid}"),
            min,
            ultima,
        }
    }

    #[test]
    fn primeira_leitura_vira_base_e_depois_conta_o_que_cresceu() {
        let mut d = Dados::default();
        let n = aplicar(&mut d, Fonte::Steam, "1", &[j("10", 600, 100), j("20", 0, 0)], 1_000_000);
        assert_eq!(n, 0);
        assert_eq!(d.steam.jogos["10"].base_ms, 600 * 60_000);
        assert!(!d.steam.jogos.contains_key("20"));
        assert!(d.auto_jogo.contains("steam:10"));
        assert!(d.sessoes.is_empty());

        // jogou mais 30 min, terminou às 1_500_000 ms
        let n = aplicar(&mut d, Fonte::Steam, "1", &[j("10", 630, 1_500)], 2_000_000);
        assert_eq!(n, 1);
        assert_eq!(d.sessoes.len(), 1);
        assert_eq!(d.sessoes[0].fim, 1_500_000);
        assert_eq!(d.sessoes[0].duracao(), 30 * 60_000);

        // jogo comprado depois de conectar: tudo é tempo novo
        let n = aplicar(&mut d, Fonte::Steam, "1", &[j("10", 630, 1_500), j("30", 20, 0)], 3_000_000);
        assert_eq!(n, 1);
        assert_eq!(d.steam.jogos["30"].base_ms, 0);
    }

    #[test]
    fn outra_conta_comeca_do_zero() {
        let mut d = Dados::default();
        aplicar(&mut d, Fonte::Steam, "1", &[j("10", 600, 0)], 1_000);
        aplicar(&mut d, Fonte::Steam, "2", &[j("99", 5, 0)], 2_000);
        assert!(!d.steam.jogos.contains_key("10"));
        assert_eq!(d.steam.jogos["99"].base_ms, 5 * 60_000);
        assert!(!d.nomes.contains_key("steam:10"));
    }

    #[test]
    fn steam_e_psn_ficam_separados() {
        let mut d = Dados::default();
        aplicar(&mut d, Fonte::Steam, "1", &[j("10", 600, 0)], 1_000);
        aplicar(&mut d, Fonte::Psn, "9", &[j("CUSA1", 120, 0)], 1_000);
        assert!(d.psn.jogos.contains_key("CUSA1"));
        assert!(d.auto_jogo.contains("psn:CUSA1"));
        remover(&mut d, Fonte::Psn);
        assert!(d.psn.jogos.is_empty());
        assert!(d.steam.jogos.contains_key("10"));
        assert_eq!(Fonte::de_pacote("psn:CUSA1"), Some(Fonte::Psn));
        assert_eq!(Fonte::de_pacote("com.app"), None);
    }

    #[test]
    fn le_json() {
        let (id, v) = ler(r#"{"id":"7656","jogos":[{"appid":730,"nome":"CS","min":12,"ultima":5}]}"#).unwrap();
        assert_eq!(id, "7656");
        assert_eq!(v[0].appid, "730");
    }
}
