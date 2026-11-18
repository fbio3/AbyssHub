//! Leaderboard: o que o app envia para o Firebase e o que recebe de volta.

use std::collections::HashMap;

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
    Tudo,
}

impl PeriodoRanking {
    pub const NOMES: [&'static str; 3] = ["Hoje", "Semana", "Tudo"];

    pub fn de(i: usize) -> Self {
        match i {
            0 => PeriodoRanking::Hoje,
            1 => PeriodoRanking::Semana,
            _ => PeriodoRanking::Tudo,
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
            PeriodoRanking::Hoje => "hoje",
            PeriodoRanking::Semana => "semana",
            PeriodoRanking::Tudo => "tudo",
        }
    }
}

/// Uma linha do Leaderboard.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
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

#[derive(Serialize, PartialEq)]
struct Top {
    pacote: String,
    nome: String,
}

#[derive(Serialize, PartialEq)]
struct JogoPub {
    pacote: String,
    nome: String,
    chave: String,
    hoje_ms: i64,
    semana_ms: i64,
    tudo_ms: i64,
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
    jogos: Vec<JogoPub>,
}

/// Monta o JSON com o tempo da pessoa (só jogos). `nome` dá o nome de cada pacote.
pub fn montar_publicacao(
    sessoes: &[Sessao],
    nome: impl Fn(&str) -> String,
    agora: i64,
    ini_hoje: i64,
    ini_semana: i64,
    dia: &str,
) -> String {
    let mut por_jogo: HashMap<&str, [i64; 3]> = HashMap::new();
    for s in sessoes {
        let e = por_jogo.entry(s.pacote.as_str()).or_insert([0; 3]);
        e[0] += sessoes::tempo_no_intervalo(s, ini_hoje, agora);
        e[1] += sessoes::tempo_no_intervalo(s, ini_semana, agora);
        e[2] += s.duracao();
    }
    let mut jogos: Vec<JogoPub> = por_jogo
        .into_iter()
        .map(|(p, t)| {
            let n = nome(p);
            JogoPub {
                chave: chave_jogo(&n, p),
                pacote: p.to_string(),
                nome: n,
                hoje_ms: t[0],
                semana_ms: t[1],
                tudo_ms: t[2],
            }
        })
        .collect();
    jogos.sort_by(|a, b| b.tudo_ms.cmp(&a.tudo_ms).then(a.pacote.cmp(&b.pacote)));
    jogos.truncate(30);

    let top = |f: fn(&JogoPub) -> i64| {
        jogos.iter().filter(|j| f(j) > 0).max_by_key(|j| f(j)).map(|j| Top {
            pacote: j.pacote.clone(),
            nome: j.nome.clone(),
        })
    };
    let p = Publicacao {
        dia: dia.to_string(),
        hoje_ms: jogos.iter().map(|j| j.hoje_ms).sum(),
        semana_ms: jogos.iter().map(|j| j.semana_ms).sum(),
        tudo_ms: jogos.iter().map(|j| j.tudo_ms).sum(),
        top_hoje: top(|j| j.hoje_ms),
        top_semana: top(|j| j.semana_ms),
        top_tudo: top(|j| j.tudo_ms),
        jogos,
    };
    serde_json::to_string(&p).unwrap_or_default()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn publicacao_soma_e_escolhe_o_top() {
        let s = vec![
            Sessao {
                pacote: "a".into(),
                inicio: 0,
                fim: 1000,
            },
            Sessao {
                pacote: "b".into(),
                inicio: 5000,
                fim: 9000,
            },
        ];
        let json = montar_publicacao(&s, |p| p.to_uppercase(), 10_000, 4000, 0, "2026-11-17");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["tudo_ms"], 5000);
        assert_eq!(v["hoje_ms"], 4000);
        assert_eq!(v["top_tudo"]["pacote"], "b");
        assert_eq!(v["top_hoje"]["nome"], "B");
        assert_eq!(v["jogos"][0]["chave"], "b");
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
