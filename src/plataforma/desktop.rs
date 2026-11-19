//! Versão de mentirinha para testar a interface no PC (`cargo run`).
//! Gera sessões de jogos fictícios para os últimos dias.

use std::path::PathBuf;
use std::sync::Mutex;

use super::{agora_ms, Conta, EstadoConta, EstadoFonte, Fonte, InfoApp, Plataforma};
use crate::sessoes::{Evento, PAUSA, RETOMADA};

const APPS: [(&str, &str, bool); 5] = [
    ("com.abyss.knight", "Abyss Knight", true),
    ("com.exemplo.corrida", "Corrida Neon", true),
    ("com.exemplo.xadrez", "Xadrez Rápido", true),
    ("com.exemplo.cartas", "Cartas do Abismo", true),
    ("com.exemplo.navegador", "Navegador", false),
];

const DIA: i64 = 86_400_000;

pub struct PlataformaDesktop {
    pasta: PathBuf,
    /// formas de entrar da conta de teste (vazio = não está logado)
    logado: Mutex<Vec<String>>,
    /// Steam e PSN de teste conectadas (índice = Fonte)
    fontes: Mutex<[bool; 2]>,
}

/// Jogos da Steam de teste: (appid, nome, minutos). "Abyss Knight" também existe no "celular"
/// e na PSN, para testar o jogo que está em várias plataformas.
const STEAM: [(&str, &str, i64); 3] = [
    ("730", "Counter-Strike 2", 5400),
    ("105600", "Terraria", 2300),
    ("999001", "Abyss Knight", 300),
];

/// Jogos da PSN de teste.
const PSN: [(&str, &str, i64); 3] = [
    ("PPSA01284", "God of War Ragnarök", 3100),
    ("PPSA03016", "Gran Turismo 7", 900),
    ("PPSA99001", "Abyss Knight", 150),
];

impl PlataformaDesktop {
    pub fn nova() -> Self {
        PlataformaDesktop {
            pasta: PathBuf::from("dados-teste"),
            logado: Mutex::new(Vec::new()),
            fontes: Mutex::new([false; 2]),
        }
    }
}

impl Plataforma for PlataformaDesktop {
    fn tem_permissao(&self) -> bool {
        true
    }

    fn abrir_permissao(&self) {
        log::info!("(PC) aqui o Android abriria a tela de Acesso ao uso");
    }

    fn listar_apps(&self) -> Result<Vec<InfoApp>, String> {
        Ok(APPS
            .iter()
            .map(|(p, n, j)| InfoApp {
                pacote: p.to_string(),
                nome: n.to_string(),
                jogo: *j,
                abrivel: true,
            })
            .collect())
    }

    fn eventos(&self, desde: i64, ate: i64) -> Result<Vec<Evento>, String> {
        let mut v = Vec::new();
        for d in desde.div_euclid(DIA)..=ate.div_euclid(DIA) {
            // Gerador simples e sempre igual para o mesmo dia.
            let mut semente = (d as u64).wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let mut rnd = |m: u64| {
                semente = semente
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                ((semente >> 33) % m) as i64
            };
            let quantas = rnd(4);
            for _ in 0..quantas {
                let pacote = APPS[rnd(APPS.len() as u64) as usize].0.to_string();
                let ini = d * DIA + (rnd(13) + 9) * 3_600_000 + rnd(60) * 60_000;
                let fim = ini + (rnd(90) + 5) * 60_000;
                if ini < desde || ini > ate {
                    continue;
                }
                v.push(Evento {
                    pacote: pacote.clone(),
                    classe: "Main".into(),
                    tipo: RETOMADA,
                    ts: ini,
                });
                if fim <= ate {
                    v.push(Evento {
                        pacote,
                        classe: "Main".into(),
                        tipo: PAUSA,
                        ts: fim,
                    });
                }
            }
        }
        Ok(v)
    }

    fn pasta_dados(&self) -> PathBuf {
        self.pasta.clone()
    }

    fn pacote_proprio(&self) -> String {
        "com.abysshub.app".into()
    }

    // ---------- Leaderboard de mentirinha (para testar a tela no PC) ----------

    fn conta(&self) -> Conta {
        let provedores = self.logado.lock().unwrap().clone();
        if !provedores.is_empty() {
            Conta {
                estado: EstadoConta::Logado,
                msg: String::new(),
                uid: "eu".into(),
                nome: "Você (teste)".into(),
                foto: String::new(),
                email: "teste@exemplo.com".into(),
                provedores,
            }
        } else {
            Conta {
                estado: EstadoConta::Saiu,
                ..Default::default()
            }
        }
    }

    fn entrar(&self) {
        *self.logado.lock().unwrap() = vec!["google.com".into()];
    }

    fn sair(&self) {
        self.logado.lock().unwrap().clear();
    }

    fn entrar_email(&self) {
        *self.logado.lock().unwrap() = vec!["password".into()];
    }

    fn ligar_google(&self) {
        let mut p = self.logado.lock().unwrap();
        if !p.is_empty() && !p.iter().any(|x| x == "google.com") {
            p.push("google.com".into());
        }
    }

    fn ligar_email(&self) {
        let mut p = self.logado.lock().unwrap();
        if !p.is_empty() && !p.iter().any(|x| x == "password") {
            p.push("password".into());
        }
    }

    fn publicar(&self, _json: &str) -> Result<(), String> {
        Ok(())
    }

    fn apagar_meus_dados(&self) -> Result<(), String> {
        Ok(())
    }

    fn fonte_estado(&self, fonte: Fonte) -> EstadoFonte {
        if self.fontes.lock().unwrap()[fonte as usize] {
            EstadoFonte {
                estado: "ok".into(),
                msg: String::new(),
                nome: format!("Você ({} de teste)", fonte.nome()),
            }
        } else {
            EstadoFonte {
                estado: "desligado".into(),
                ..Default::default()
            }
        }
    }

    fn fonte_conectar(&self, fonte: Fonte) {
        self.fontes.lock().unwrap()[fonte as usize] = true;
    }

    fn fonte_desconectar(&self, fonte: Fonte) {
        self.fontes.lock().unwrap()[fonte as usize] = false;
    }

    fn fonte_jogos(&self, fonte: Fonte) -> Result<Option<String>, String> {
        if !self.fontes.lock().unwrap()[fonte as usize] {
            return Ok(None);
        }
        // o primeiro jogo "ganha" 1 minuto a cada minuto, para dar para ver o tempo novo aparecer
        let minuto = agora_ms() / 60_000;
        let lista = match fonte {
            Fonte::Steam => &STEAM,
            Fonte::Psn => &PSN,
        };
        let jogos: Vec<serde_json::Value> = lista
            .iter()
            .enumerate()
            .map(|(i, (id, nome, min))| {
                let extra = if i == 1 { minuto % 100_000 } else { 0 };
                serde_json::json!({
                    "appid": id, "nome": nome, "min": min + extra,
                    "ultima": agora_ms() / 1000 - 60,
                })
            })
            .collect();
        let id = format!("{}-teste", fonte.chave());
        Ok(Some(serde_json::json!({ "id": id, "jogos": jogos }).to_string()))
    }

    fn buscar_ranking(
        &self,
        modo: &str,
        periodo: &str,
        plataforma: &str,
        pacote: &str,
        _dia: &str,
    ) -> Result<String, String> {
        let pessoas = [
            ("eu", "Você (teste)"),
            ("a", "Riko"),
            ("b", "Reg Explorador"),
            ("c", "Nanachi"),
            ("d", "Ozen"),
            ("e", "Maruruk"),
            ("f", "Prushka"),
            ("g", "Bondrewd"),
            ("h", "Faputa"),
        ];
        let fator = match periodo {
            "hoje" => 1,
            "semana" => 6,
            "mes" => 20,
            _ => 40,
        };
        // cada jogo de teste "existe" em: celular, Steam, PSN ou em várias
        let plats_de = |j: usize| -> Vec<&'static str> {
            match j {
                0 => vec!["mobile", "steam", "psn"],
                2 => vec!["psn"],
                3 => vec!["steam"],
                _ => vec!["mobile"],
            }
        };
        // muda um pouquinho a cada minuto, para dar para ver a atualização
        let minuto = (agora_ms() / 60_000) as i64;
        let mut itens = Vec::new();
        for (i, (uid, nome)) in pessoas.iter().enumerate() {
            let jogos: Vec<usize> = match modo {
                "pares" => vec![i % 4, (i + 1) % 4],
                "jogo" => APPS
                    .iter()
                    .position(|a| crate::ranking::chave_jogo(a.1, a.0) == pacote)
                    .map(|p| vec![p])
                    .unwrap_or_default(),
                _ => vec![i % 4],
            };
            for (k, j) in jogos.into_iter().enumerate() {
                let pl = plats_de(j);
                if plataforma != "todas" && !pl.iter().any(|p| *p == plataforma) {
                    continue;
                }
                let base = ((9 - i) as i64 * 37 + k as i64 * 11 + (minuto + i as i64) % 7) * 60_000;
                // jogo em várias plataformas: o tempo é dividido entre elas
                let parte = match (plataforma, pl.len()) {
                    ("todas", _) | (_, 1) => 100,
                    ("mobile", _) => 50,
                    ("steam", _) => 30,
                    _ => 20,
                };
                let ms = base * fator / 3 * parte / 100;
                itens.push(serde_json::json!({
                    "uid": uid, "nome": nome, "foto": "",
                    "pacote": APPS[j].0, "jogo": APPS[j].1,
                    "chave": crate::ranking::chave_jogo(APPS[j].1, APPS[j].0),
                    "ms": ms, "atualizado_ms": agora_ms() - (i as i64) * 90_000,
                    "plats": pl,
                }));
            }
        }
        itens.sort_by_key(|v| -v["ms"].as_i64().unwrap_or(0));
        Ok(serde_json::Value::Array(itens).to_string())
    }
}
