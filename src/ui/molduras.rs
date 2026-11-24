//! Tela "Moldura da foto": mostra todas as molduras, deixa tocar em cada uma para ver como fica
//! na sua foto e usar as que você já ganhou. As que você não tem aparecem mais escuras e com cadeado.

use std::sync::mpsc::{self, TryRecvError};

use eframe::egui::{self, vec2, Align2, CornerRadius, Sense, Stroke, StrokeKind, Ui};

use super::desenho::{self, Icone};
use super::telas::botao;
use super::tema::{alfa, fonte, txt, Peso};
use super::*;
use crate::molduras;
use crate::plataforma::EstadoConta;
use crate::ranking::ItemRanking;

impl AbyssHubApp {
    pub(crate) fn abrir_molduras(&mut self) {
        self.ajustes_aberto = false;
        self.molduras_aberta = true;
        self.moldura_prev = None;
        self.trocar_pagina(1.0);
    }

    pub(crate) fn fechar_molduras(&mut self) {
        self.molduras_aberta = false;
        self.moldura_prev = None;
        self.trocar_pagina(-1.0);
    }

    fn receber_moldura(&mut self) {
        let Some(rx) = &self.rx_moldura else { return };
        let (id, r) = match rx.try_recv() {
            Ok(x) => x,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.rx_moldura = None;
                return;
            }
        };
        self.rx_moldura = None;
        match r {
            Ok(()) => {
                self.conta.moldura = id.clone();
                self.ultimo_check_conta = None;
                // manda a moldura nova para o Leaderboard, o Placar e os Eventos
                self.ultima_publicacao = None;
                self.hist_publicado = None;
                self.moldura_prev = None;
                let msg = if id.is_empty() {
                    "Moldura removida".to_string()
                } else {
                    format!("Agora você usa a moldura {}", molduras::nome(&id))
                };
                self.mostrar_toast(msg, false);
            }
            Err(e) => self.mostrar_toast(e, true),
        }
    }

    /// A sua foto com a moldura `id` ("" = sem moldura).
    fn sua_foto(&self, id: &str) -> ItemRanking {
        ItemRanking {
            uid: self.conta.uid.clone(),
            nome: self.conta.nome.clone(),
            foto: self.conta.foto.clone(),
            moldura: id.to_string(),
            ..Default::default()
        }
    }

    pub(super) fn tela_molduras(&mut self, ui: &mut Ui, topo: f32) {
        self.receber_moldura();
        let logado = self.conta.estado == EstadoConta::Logado;
        let usando = self.conta.moldura.clone();
        let tem = self.conta.molduras.clone();
        let sel = self.moldura_prev.clone().unwrap_or_else(|| usando.clone());
        let tem_sel = sel.is_empty() || tem.iter().any(|m| *m == sel);

        // ordem: sem moldura, as que você tem (da mais rara), depois as que faltam
        let mut lista: Vec<&'static str> = vec![""];
        for m in molduras::TODAS.iter().filter(|m| tem.iter().any(|t| t == m.0)) {
            lista.push(m.0);
        }
        for m in molduras::TODAS.iter().filter(|m| !tem.iter().any(|t| t == m.0)) {
            lista.push(m.0);
        }

        self.hero(ui, topo, |s, ui| {
            ui.horizontal(|ui| {
                if desenho::botao_icone(ui, &mut s.an, &s.p, "voltar_molduras", Icone::Voltar).clicked() {
                    s.fechar_molduras();
                }
                ui.add_space(2.0);
                ui.label(txt("Moldura da foto", 19.0, Peso::Negrito, s.p.on_hero));
            });
            ui.add_space(8.0);
            // a sua foto grande com a moldura escolhida
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 132.0), Sense::hover());
            let pt = ui.painter().clone();
            let pop = s.an.desde(s.inicio_dados, 0.0, 0.45);
            let raio = 46.0 + 6.0 * pop;
            let item = s.sua_foto(&sel);
            s.foto_pessoa(&pt, &item, r.center(), raio);
            if !tem_sel {
                pt.circle_filled(r.center(), raio, alfa(Color32::BLACK, 0.25));
            }
            ui.vertical_centered(|ui| {
                let nome = if sel.is_empty() { "Sem moldura" } else { molduras::nome(&sel) };
                ui.label(txt(nome, 24.0, Peso::Pesado, s.p.on_hero));
                let sub = if sel.is_empty() {
                    "Sua foto sem borda.".to_string()
                } else if tem_sel {
                    molduras::descricao(&sel).to_string()
                } else {
                    format!("{} · você ainda não tem", molduras::descricao(&sel))
                };
                ui.label(txt(sub, 13.0, Peso::Normal, alfa(s.p.on_hero, 0.8)));
            });
        });

        self.conteudo(ui, |s, ui| {
            if !logado {
                s.cartao(ui, 0, |s, ui| s.cartao_login(ui));
                return;
            }
            // botão de usar
            s.cartao(ui, 0, |s, ui| {
                let p = s.p;
                if sel == usando {
                    ui.label(txt(
                        if sel.is_empty() { "Você está sem moldura." } else { "Você está usando esta moldura." },
                        14.0,
                        Peso::Semi,
                        p.text,
                    ));
                    ui.label(txt(
                        "Toque em uma moldura abaixo para ver como fica na sua foto.",
                        12.5,
                        Peso::Normal,
                        p.muted,
                    ));
                } else if tem_sel {
                    let rotulo = if sel.is_empty() { "Tirar a moldura" } else { "Usar esta moldura" };
                    let ocupado = s.rx_moldura.is_some();
                    if botao(ui, &mut s.an, &p, "usar_moldura", if ocupado { "Salvando..." } else { rotulo }, true)
                        .clicked()
                        && !ocupado
                    {
                        let plat = s.plat.clone();
                        let id = sel.clone();
                        let (tx, rx) = mpsc::channel();
                        s.rx_moldura = Some(rx);
                        std::thread::spawn(move || {
                            let r = plat.usar_moldura(&id);
                            let _ = tx.send((id, r));
                        });
                    }
                } else {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                        desenho::cadeado(ui.painter(), r.center(), 20.0, p.muted);
                        ui.label(txt("Moldura bloqueada", 14.0, Peso::Semi, p.text));
                    });
                    ui.label(txt(
                        "Ganhe esta moldura ficando entre os primeiros de um evento que tenha ela como recompensa.",
                        12.5,
                        Peso::Normal,
                        p.muted,
                    ));
                }
            });

            // todas as molduras, 2 por linha
            s.cartao(ui, 1, |s, ui| {
                s.titulo_cartao(ui, "Molduras", &format!("você tem {} de {}", tem.len(), molduras::TODAS.len()));
                for linha in lista.chunks(2) {
                    ui.columns(2, |c| {
                        for (k, id) in linha.iter().enumerate() {
                            let tem_esta = id.is_empty() || tem.iter().any(|t| t == id);
                            if s.quadro_moldura(&mut c[k], id, tem_esta, *id == sel, *id == usando) {
                                s.moldura_prev = Some(id.to_string());
                                s.inicio_dados = s.an.agora;
                            }
                        }
                    });
                    ui.add_space(6.0);
                }
            });
        });
    }

    /// Um quadrinho com a sua foto e a moldura. Devolve true se tocou.
    fn quadro_moldura(&mut self, ui: &mut Ui, id: &str, tem: bool, selecionado: bool, usando: bool) -> bool {
        let p = self.p;
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::click());
        let pt = ui.painter().clone();
        let sel = self.an.ir(&format!("mold_sel:{id}"), if selecionado { 1.0 } else { 0.0 }, 0.25);
        pt.rect_filled(r, CornerRadius::same(16), p.chip);
        if sel > 0.01 {
            pt.rect_stroke(
                r.shrink(1.0),
                CornerRadius::same(16),
                Stroke::new(2.0, alfa(p.gold, sel)),
                StrokeKind::Inside,
            );
        }
        let centro = r.center_top() + vec2(0.0, 58.0);
        let raio = 30.0;
        let item = self.sua_foto(id);
        self.foto_pessoa(&pt, &item, centro, raio);
        if !tem {
            // mais escura e com cadeado
            pt.circle_filled(centro, raio + 9.0, alfa(Color32::BLACK, 0.5));
            pt.circle_filled(centro, 14.0, alfa(Color32::BLACK, 0.55));
            desenho::cadeado(&pt, centro, 20.0, Color32::WHITE);
        }
        let nome = if id.is_empty() { "Sem moldura" } else { molduras::nome(id) };
        pt.text(
            r.center_top() + vec2(0.0, 112.0),
            Align2::CENTER_CENTER,
            nome,
            fonte(14.0, Peso::Semi),
            if tem { p.text } else { p.muted },
        );
        let (status, cor) = if usando {
            ("Usando", p.gold)
        } else if tem {
            ("Você tem", p.muted)
        } else {
            ("Bloqueada", p.muted)
        };
        pt.text(
            r.center_top() + vec2(0.0, 132.0),
            Align2::CENTER_CENTER,
            status,
            fonte(11.5, Peso::Normal),
            cor,
        );
        resp.clicked()
    }
}
