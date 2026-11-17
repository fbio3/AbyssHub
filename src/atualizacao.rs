//! Atualização automática pelo GitHub Releases (igual ao AbyssSub):
//! o app vê se tem versão nova, baixa o APK e abre o instalador.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// Repositório onde as versões são publicadas.
pub const REPO: &str = "fbio3/AbyssHub";
/// Procura sozinho no máximo a cada 6 horas.
pub const INTERVALO_MS: i64 = 6 * 3600 * 1000;

/// Uma versão publicada no GitHub.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Versao {
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub notas: String,
    #[serde(default)]
    pub pagina: String,
    /// Link do APK (vazio se a versão ainda não tem o APK anexado).
    #[serde(default)]
    pub apk: String,
    #[serde(default)]
    pub tamanho: i64,
}

/// O que fica salvo no celular sobre as atualizações.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EstadoSalvo {
    #[serde(default)]
    pub ultima_verificacao: i64,
    #[serde(default)]
    pub ultima: Option<Versao>,
    /// Versão em que a pessoa tocou "Agora não".
    #[serde(default)]
    pub dispensada: String,
    /// Versão que já foi oferecida sozinha (para não mostrar o aviso toda hora).
    #[serde(default)]
    pub ofertada: String,
}

impl EstadoSalvo {
    /// A versão publicada, se for mais nova que a instalada.
    pub fn mais_nova(&self, atual: &str) -> Option<&Versao> {
        self.ultima
            .as_ref()
            .filter(|v| !v.tag.is_empty() && comparar(&v.tag, atual) == Ordering::Greater)
    }
}

/// Andamento do download/instalação (vem do Android).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Baixa {
    /// "parado" | "baixando" | "baixado" | "permissao" | "instalando" | "erro"
    pub estado: String,
    pub progresso: i32,
    pub msg: String,
}

fn partes(v: &str) -> Vec<u64> {
    v.trim_start_matches(|c: char| !c.is_ascii_digit())
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

/// Compara "v1.2.0" com "1.10" do jeito certo (número por número).
pub fn comparar(a: &str, b: &str) -> Ordering {
    let (x, y) = (partes(a), partes(b));
    for i in 0..x.len().max(y.len()) {
        let o = x.get(i).unwrap_or(&0).cmp(y.get(i).unwrap_or(&0));
        if o != Ordering::Equal {
            return o;
        }
    }
    Ordering::Equal
}

pub fn ler_versao(json: &str) -> Result<Versao, String> {
    serde_json::from_str(json).map_err(|e| format!("Resposta estranha do GitHub: {e}"))
}

pub fn ler_baixa(json: &str) -> Baixa {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    Baixa {
        estado: v.get("estado").and_then(|x| x.as_str()).unwrap_or("parado").to_string(),
        progresso: v.get("progresso").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
        msg: v.get("msg").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    }
}

pub fn sem_v(tag: &str) -> &str {
    tag.trim_start_matches(['v', 'V'])
}

/// Tira a marcação do texto da versão (títulos, negrito, listas) e limita o tamanho.
pub fn notas_limpas(notas: &str) -> String {
    let mut s: String = notas
        .replace('\r', "")
        .lines()
        .map(|l| {
            let l = l.trim_start_matches('#').trim_start();
            let l = if let Some(r) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
                format!("• {r}")
            } else {
                l.to_string()
            };
            l.replace("**", "")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if s.chars().count() > 1200 {
        s = s.chars().take(1200).collect::<String>() + "…";
    }
    s
}

pub fn fmt_tamanho(bytes: i64) -> String {
    if bytes <= 0 {
        return String::new();
    }
    format!("{:.1} MB", bytes as f64 / 1_048_576.0).replace('.', ",")
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn compara_versoes() {
        assert_eq!(comparar("v1.0.1", "1.0.0"), Ordering::Greater);
        assert_eq!(comparar("1.10", "1.9.9"), Ordering::Greater);
        assert_eq!(comparar("v1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(comparar("0.9", "1.0.0"), Ordering::Less);
    }

    #[test]
    fn so_oferece_se_for_mais_nova() {
        let mut e = EstadoSalvo::default();
        assert!(e.mais_nova("1.0.0").is_none());
        e.ultima = Some(Versao {
            tag: "v1.0.0".into(),
            ..Default::default()
        });
        assert!(e.mais_nova("1.0.0").is_none());
        e.ultima = Some(Versao {
            tag: "v1.1.0".into(),
            ..Default::default()
        });
        assert_eq!(e.mais_nova("1.0.0").unwrap().tag, "v1.1.0");
    }

    #[test]
    fn limpa_notas() {
        assert_eq!(
            notas_limpas("## Novidades\r\n- **Leaderboard** novo"),
            "Novidades\n• Leaderboard novo"
        );
        assert_eq!(fmt_tamanho(7_654_321), "7,3 MB");
    }
}
