//! Transforma os eventos de uso do Android em sessões de jogo (início/fim).
//!
//! Este módulo não depende de nada do Android, então dá para testar no PC:
//!   rustc --edition 2021 --test --cfg teste_isolado src/sessoes.rs -o teste && ./teste

use std::collections::BTreeSet;

// Tipos de evento do UsageEvents.Event (valores do Android).
pub const RETOMADA: i32 = 1; // ACTIVITY_RESUMED (antes MOVE_TO_FOREGROUND)
pub const PAUSA: i32 = 2; // ACTIVITY_PAUSED (antes MOVE_TO_BACKGROUND)
pub const TELA_DESLIGADA: i32 = 16; // SCREEN_NON_INTERACTIVE
pub const BLOQUEIO: i32 = 17; // KEYGUARD_SHOWN
pub const PARADA: i32 = 23; // ACTIVITY_STOPPED
pub const DESLIGAMENTO: i32 = 26; // DEVICE_SHUTDOWN

/// Duas sessões do mesmo app separadas por menos que isso viram uma só.
pub const JUNTAR_MS: i64 = 30_000;
/// Sessões menores que isso são ignoradas (abriu e fechou sem querer).
pub const MIN_SESSAO_MS: i64 = 5_000;

pub fn evento_relevante(tipo: i32) -> bool {
    matches!(
        tipo,
        RETOMADA | PAUSA | PARADA | TELA_DESLIGADA | BLOQUEIO | DESLIGAMENTO
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct Evento {
    pub pacote: String,
    pub classe: String,
    pub tipo: i32,
    pub ts: i64,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(not(teste_isolado), derive(serde::Serialize, serde::Deserialize))]
pub struct Sessao {
    pub pacote: String,
    pub inicio: i64,
    pub fim: i64,
}

impl Sessao {
    pub fn duracao(&self) -> i64 {
        (self.fim - self.inicio).max(0)
    }
}

#[derive(Debug, Default)]
pub struct Resultado {
    /// Sessões que começaram e terminaram dentro do intervalo lido.
    pub fechadas: Vec<Sessao>,
    /// App que ainda estava aberto no fim do intervalo: (pacote, início).
    pub aberta: Option<(String, i64)>,
}

struct Atual {
    pacote: String,
    inicio: i64,
    telas: BTreeSet<String>,
}

/// Monta as sessões a partir dos eventos (em qualquer ordem).
pub fn montar_sessoes(eventos: &[Evento]) -> Resultado {
    let mut ordenados: Vec<&Evento> = eventos.iter().collect();
    ordenados.sort_by_key(|e| e.ts);

    let mut atual: Option<Atual> = None;
    let mut fechadas: Vec<Sessao> = Vec::new();

    let fechar = |a: Atual, fim: i64, fechadas: &mut Vec<Sessao>| {
        if fim > a.inicio {
            fechadas.push(Sessao {
                pacote: a.pacote,
                inicio: a.inicio,
                fim,
            });
        }
    };

    for e in ordenados {
        match e.tipo {
            RETOMADA => {
                let mesmo_app = atual.as_ref().map_or(false, |a| a.pacote == e.pacote);
                if mesmo_app {
                    if let Some(a) = atual.as_mut() {
                        a.telas.insert(e.classe.clone());
                    }
                } else {
                    if let Some(a) = atual.take() {
                        fechar(a, e.ts, &mut fechadas);
                    }
                    let mut telas = BTreeSet::new();
                    telas.insert(e.classe.clone());
                    atual = Some(Atual {
                        pacote: e.pacote.clone(),
                        inicio: e.ts,
                        telas,
                    });
                }
            }
            PAUSA | PARADA => {
                let mut terminou = false;
                if let Some(a) = atual.as_mut() {
                    if a.pacote == e.pacote {
                        a.telas.remove(&e.classe);
                        terminou = a.telas.is_empty();
                    }
                }
                if terminou {
                    if let Some(a) = atual.take() {
                        fechar(a, e.ts, &mut fechadas);
                    }
                }
            }
            TELA_DESLIGADA | BLOQUEIO | DESLIGAMENTO => {
                if let Some(a) = atual.take() {
                    fechar(a, e.ts, &mut fechadas);
                }
            }
            _ => {}
        }
    }

    let mut juntas = Vec::new();
    for s in fechadas {
        mesclar(&mut juntas, s);
    }

    Resultado {
        fechadas: juntas,
        aberta: atual.map(|a| (a.pacote, a.inicio)),
    }
}

/// Insere uma sessão na lista (ordenada por início), juntando com sessões do
/// mesmo app que se sobrepõem ou estão a menos de JUNTAR_MS de distância.
/// Também serve para não duplicar quando o mesmo trecho é lido duas vezes.
pub fn mesclar(lista: &mut Vec<Sessao>, nova: Sessao) {
    let mut nova = nova;
    let mut i = 0;
    while i < lista.len() {
        let s = &lista[i];
        if s.pacote == nova.pacote && s.inicio <= nova.fim + JUNTAR_MS && nova.inicio <= s.fim + JUNTAR_MS {
            let s = lista.remove(i);
            nova.inicio = nova.inicio.min(s.inicio);
            nova.fim = nova.fim.max(s.fim);
        } else {
            i += 1;
        }
    }
    let pos = lista.partition_point(|s| s.inicio <= nova.inicio);
    lista.insert(pos, nova);
}

/// Quanto da sessão cai dentro de [a, b).
pub fn tempo_no_intervalo(s: &Sessao, a: i64, b: i64) -> i64 {
    (s.fim.min(b) - s.inicio.max(a)).max(0)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ev(p: &str, c: &str, tipo: i32, ts: i64) -> Evento {
        Evento {
            pacote: p.into(),
            classe: c.into(),
            tipo,
            ts,
        }
    }

    #[test]
    fn sessao_simples() {
        let r = montar_sessoes(&[ev("jogo", "Main", RETOMADA, 1000), ev("jogo", "Main", PAUSA, 61_000)]);
        assert_eq!(
            r.fechadas,
            vec![Sessao {
                pacote: "jogo".into(),
                inicio: 1000,
                fim: 61_000
            }]
        );
        assert!(r.aberta.is_none());
    }

    #[test]
    fn trocar_de_app_fecha_o_anterior() {
        let r = montar_sessoes(&[
            ev("a", "M", RETOMADA, 0),
            ev("b", "M", RETOMADA, 100_000),
            ev("b", "M", PAUSA, 200_000),
        ]);
        assert_eq!(r.fechadas.len(), 2);
        assert_eq!(r.fechadas[0].pacote, "a");
        assert_eq!(r.fechadas[0].fim, 100_000);
        assert_eq!(r.fechadas[1].duracao(), 100_000);
    }

    #[test]
    fn desligar_tela_fecha() {
        let r = montar_sessoes(&[ev("a", "M", RETOMADA, 0), ev("android", "", TELA_DESLIGADA, 50_000)]);
        assert_eq!(r.fechadas[0].fim, 50_000);
        assert!(r.aberta.is_none());
    }

    #[test]
    fn varias_telas_do_mesmo_app() {
        let r = montar_sessoes(&[
            ev("a", "Menu", RETOMADA, 0),
            ev("a", "Fase", RETOMADA, 10_000),
            ev("a", "Menu", PAUSA, 10_001),
            ev("a", "Fase", PAUSA, 90_000),
        ]);
        assert_eq!(
            r.fechadas,
            vec![Sessao {
                pacote: "a".into(),
                inicio: 0,
                fim: 90_000
            }]
        );
    }

    #[test]
    fn pausa_e_volta_rapida_vira_uma_sessao() {
        let r = montar_sessoes(&[
            ev("a", "M", RETOMADA, 0),
            ev("a", "M", PAUSA, 60_000),
            ev("a", "M", RETOMADA, 70_000),
            ev("a", "M", PAUSA, 120_000),
        ]);
        assert_eq!(r.fechadas.len(), 1);
        assert_eq!(r.fechadas[0].duracao(), 120_000);
    }

    #[test]
    fn app_ainda_aberto() {
        let r = montar_sessoes(&[ev("a", "M", RETOMADA, 5)]);
        assert!(r.fechadas.is_empty());
        assert_eq!(r.aberta, Some(("a".into(), 5)));
    }

    #[test]
    fn eventos_fora_de_ordem() {
        let r = montar_sessoes(&[ev("a", "M", PAUSA, 60_000), ev("a", "M", RETOMADA, 0)]);
        assert_eq!(r.fechadas[0].duracao(), 60_000);
    }

    #[test]
    fn mesclar_nao_duplica() {
        let mut l = Vec::new();
        mesclar(
            &mut l,
            Sessao {
                pacote: "a".into(),
                inicio: 0,
                fim: 100_000,
            },
        );
        mesclar(
            &mut l,
            Sessao {
                pacote: "a".into(),
                inicio: 0,
                fim: 100_000,
            },
        );
        mesclar(
            &mut l,
            Sessao {
                pacote: "b".into(),
                inicio: 50_000,
                fim: 60_000,
            },
        );
        mesclar(
            &mut l,
            Sessao {
                pacote: "a".into(),
                inicio: 500_000,
                fim: 600_000,
            },
        );
        assert_eq!(l.len(), 3);
        assert!(l.windows(2).all(|w| w[0].inicio <= w[1].inicio));
    }

    #[test]
    fn intervalo() {
        let s = Sessao {
            pacote: "a".into(),
            inicio: 100,
            fim: 200,
        };
        assert_eq!(tempo_no_intervalo(&s, 150, 1000), 50);
        assert_eq!(tempo_no_intervalo(&s, 300, 1000), 0);
        assert_eq!(tempo_no_intervalo(&s, 0, 1000), 100);
    }
}
