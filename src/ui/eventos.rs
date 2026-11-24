//! Aba Eventos: eventos criados pelo criador do app (nome, descrição, recompensas e período).
//! Mostra quem está na frente enquanto o evento acontece e os vencedores quando acaba.
//! Só a conta do criador vê os botões de criar, editar e apagar.

use eframe::egui::{self, vec2, Align, CornerRadius, Frame, Layout, Margin, Sense, Ui};

use super::desenho::{self, Icone};
use super::telas::botao;
use super::tema::{alfa, fonte, txt, Peso};
use super::*;
use crate::historico::{self, Evento, Situacao};
use crate::plataforma::{agora_ms, EstadoConta};
use crate::ranking::PlataformaRanking;

const VERDE: Color32 = Color32::from_rgb(0x2e, 0xa0, 0x43);

/// "3 dias e 4 h", "5 h e 20 min", "12 min"
fn fmt_falta(ms: i64) -> String {
    let min = (ms / 60_000).max(1);
    let (d, h, m) = (min / (60 * 24), (min / 60) % 24, min % 60);
    if d > 0 {
        let dias = if d == 1 { "1 dia".to_string() } else { format!("{d} dias") };
        if h > 0 && d < 3 {
            format!("{dias} e {h} h")
        } else {
            dias
        }
    } else if h > 0 {
        if m > 0 {
            format!("{h} h e {m} min")
        } else {
            format!("{h} h")
        }
    } else {
        format!("{m} min")
    }
}

impl AbyssHubApp {
    pub(super) fn tela_eventos(&mut self, ui: &mut Ui, topo: f32) {
        let logado = self.conta.estado == EstadoConta::Logado;
        let criador = logado && self.conta.criador;
        let agora = agora_ms();
        let hoje = historico::hoje();
        let eu = self.conta.uid.clone();

        let mut ao_vivo: Vec<Evento> = Vec::new();
        let mut futuros: Vec<Evento> = Vec::new();
        let mut terminados: Vec<Evento> = Vec::new();
        for e in &self.eventos {
            match e.situacao(agora) {
                Situacao::Andamento => ao_vivo.push(e.clone()),
                Situacao::Futuro => futuros.push(e.clone()),
                Situacao::Terminado => terminados.push(e.clone()),
            }
        }
        // ao vivo: o que acaba primeiro; em breve: o que começa primeiro; terminados: o mais recente
        ao_vivo.sort_by_key(|e| if e.fim_ms == 0 { i64::MAX } else { e.fim_ms });
        futuros.sort_by_key(|e| e.inicio_ms);
        terminados.sort_by_key(|e| -e.fim_ms);

        let vitorias = terminados
            .iter()
            .filter(|e| {
                !eu.is_empty()
                    && e.ranking(&self.hist, hoje)
                        .iter()
                        .take(e.qtd_vencedores())
                        .any(|i| i.uid == eu)
            })
            .count();
        let carregando = self.ev_quando.is_none() && self.ev_erro.is_none();

        self.hero(ui, topo, |s, ui| {
            s.barra_marca(ui, Some("Eventos"));
            if !logado {
                s.rotulo_hero(ui, "Eventos do conjunto Abyss");
                ui.label(txt("Desafios e prêmios", 32.0, Peso::Pesado, s.p.on_hero));
                s.sub_hero(ui, "Entre na sua conta para participar dos eventos.");
                return;
            }
            let pop = s.an.desde(s.inicio_dados, 0.1, 0.5);
            match ao_vivo.first() {
                Some(e) => {
                    s.rotulo_hero(ui, "Acontecendo agora");
                    ui.add(egui::Label::new(txt(&e.nome, 28.0 + 4.0 * pop, Peso::Pesado, s.p.on_hero)).truncate());
                    let sub = if e.fim_ms > 0 {
                        format!("Termina em {}", fmt_falta(e.fim_ms - agora))
                    } else {
                        "Sem data para acabar".to_string()
                    };
                    s.sub_hero(ui, &sub);
                }
                None => {
                    s.rotulo_hero(ui, "Acontecendo agora");
                    let t = if carregando { "Carregando..." } else { "Nenhum evento" };
                    ui.label(txt(t, 28.0 + 4.0 * pop, Peso::Pesado, s.p.on_hero));
                    let sub = match futuros.first() {
                        Some(e) => format!("Próximo: {} · começa em {}", e.nome, fmt_falta(e.inicio_ms - agora)),
                        None => "Fique de olho: quando tiver evento, você recebe um aviso.".to_string(),
                    };
                    s.sub_hero(ui, &sub);
                }
            }
            s.pilulas(
                ui,
                [
                    ("Ao vivo · em breve", format!("{} · {}", ao_vivo.len(), futuros.len())),
                    ("Suas vitórias", vitorias.to_string()),
                ],
            );
        });

        self.conteudo(ui, |s, ui| {
            if !logado {
                s.cartao(ui, 0, |s, ui| s.cartao_login(ui));
                return;
            }
            let mut i = 0;
            if criador {
                s.cartao(ui, i, |s, ui| {
                    s.titulo_cartao(ui, "Criador", "só você vê isto");
                    ui.label(txt(
                        "Crie eventos com nome, descrição, recompensas e o período. Pode ser no passado, agora ou no futuro.",
                        13.0,
                        Peso::Normal,
                        s.p.muted,
                    ));
                    if botao(ui, &mut s.an, &s.p, "ev_criar", "Criar evento", true).clicked() {
                        let jogos = s.jogos_json();
                        s.plat.evento_editar("", &jogos);
                    }
                });
                i += 1;
            }
            if let Some(e) = s.ev_erro.clone() {
                s.cartao(ui, i, |s, ui| {
                    ui.label(txt("Algo deu errado", 15.0, Peso::Semi, s.p.bad));
                    ui.label(txt(&e, 12.5, Peso::Normal, s.p.muted));
                    if botao(ui, &mut s.an, &s.p, "ev_tentar", "Tentar de novo", false).clicked() {
                        s.ev_pedido = None;
                        s.hist_pedido = None;
                        s.ev_erro = None;
                    }
                });
                i += 1;
            }
            if ao_vivo.is_empty() && futuros.is_empty() && terminados.is_empty() {
                s.cartao(ui, i, |s, ui| {
                    if carregando {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.add(egui::Spinner::new().size(28.0));
                            ui.label(txt("Buscando os eventos...", 13.5, Peso::Normal, s.p.muted));
                        });
                    } else {
                        s.vazio(ui, "Nenhum evento ainda", "Quando tiver um evento, ele aparece aqui.");
                    }
                });
                return;
            }
            for (titulo, lista, sit) in [
                ("Acontecendo agora", &ao_vivo, Situacao::Andamento),
                ("Em breve", &futuros, Situacao::Futuro),
                ("Terminados", &terminados, Situacao::Terminado),
            ] {
                if lista.is_empty() {
                    continue;
                }
                ui.label(
                    txt(titulo.to_uppercase(), 12.0, Peso::Negrito, s.p.muted).extra_letter_spacing(1.0),
                );
                for e in lista.iter() {
                    s.cartao(ui, i, |s, ui| s.cartao_evento(ui, e, sit, criador, agora));
                    i += 1;
                }
            }
        });
    }

    /// Escolher as molduras de cada colocação (pode marcar várias por lugar). Só o criador.
    fn editor_premios(&mut self, ui: &mut Ui, e: &Evento) {
        self.receber_premios();
        let p = self.p;
        let t = self.an.agora;
        let Some((_, mut mapa)) = self.ev_premios_edit.clone() else { return };
        let mut mudou = false;
        let mut animada = false;
        Frame::default()
            .fill(p.chip)
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(txt("Molduras de recompensa", 14.0, Peso::Negrito, p.text));
                ui.label(txt(
                    "Toque para marcar ou desmarcar. Cada lugar pode ganhar mais de uma.",
                    12.0,
                    Peso::Normal,
                    p.muted,
                ));
                for pos in 0..e.qtd_vencedores() {
                    let chave = (pos + 1).to_string();
                    ui.add_space(4.0);
                    ui.label(txt(format!("{}º lugar", pos + 1), 13.0, Peso::Semi, p.text));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for (id, nome, _) in crate::molduras::TODAS {
                            let marcada = mapa.get(&chave).is_some_and(|v| v.iter().any(|m| m == id));
                            let g = ui.painter().layout_no_wrap(nome.to_string(), fonte(12.5, Peso::Semi), p.text);
                            let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 44.0, 34.0), Sense::click());
                            let pt = ui.painter().clone();
                            pt.rect_filled(r, CornerRadius::same(17), if marcada { p.chip_on } else { p.card });
                            let c = r.left_center() + vec2(18.0, 0.0);
                            pt.circle_filled(c, 9.0, p.chip);
                            animada |= desenho::moldura(&pt, id, c, 9.0, t, if marcada { 1.0 } else { 0.5 });
                            pt.galley(
                                r.left_center() + vec2(34.0, -g.size().y / 2.0),
                                g,
                                if marcada { p.chip_on_ink } else { p.text },
                            );
                            if resp.clicked() {
                                let v = mapa.entry(chave.clone()).or_default();
                                if marcada {
                                    v.retain(|m| m != id);
                                } else {
                                    v.push(id.to_string());
                                }
                                if v.is_empty() {
                                    mapa.remove(&chave);
                                }
                                mudou = true;
                            }
                        }
                    });
                }
                ui.add_space(6.0);
                let salvando = self.rx_ev_premios.is_some();
                let mut salvar = false;
                let mut cancelar = false;
                ui.columns(2, |c| {
                    salvar = botao(&mut c[0], &mut self.an, &p, "ev_pr_salvar", if salvando { "Salvando..." } else { "Salvar" }, true)
                        .clicked();
                    cancelar = botao(&mut c[1], &mut self.an, &p, "ev_pr_cancelar", "Cancelar", false).clicked();
                });
                if salvar && !salvando {
                    let json = serde_json::to_string(&mapa).unwrap_or_else(|_| "{}".into());
                    let plat = self.plat.clone();
                    let id = e.id.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.rx_ev_premios = Some(rx);
                    std::thread::spawn(move || {
                        let _ = tx.send(plat.evento_premios(&id, &json));
                    });
                }
                if cancelar {
                    self.ev_premios_edit = None;
                }
            });
        if mudou {
            if let Some((_, m)) = &mut self.ev_premios_edit {
                *m = mapa;
            }
        }
        if animada {
            self.an.ativo = true;
        }
    }

    fn receber_premios(&mut self) {
        let Some(rx) = &self.rx_ev_premios else { return };
        let r = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err("Não deu para salvar".into()),
        };
        self.rx_ev_premios = None;
        match r {
            Ok(()) => {
                self.ev_premios_edit = None;
                self.ev_pedido = None; // recarrega os eventos
                self.mostrar_toast("Recompensas salvas", false);
            }
            Err(e) => self.mostrar_toast(e, true),
        }
    }

    /// Jogos para o formulário de evento: [[chave, nome], ...]
    fn jogos_json(&self) -> String {
        let v: Vec<(String, String)> = self.jogos_placar().into_iter().map(|(c, _, n)| (c, n)).collect();
        serde_json::to_string(&v).unwrap_or_else(|_| "[]".into())
    }

    fn cartao_evento(&mut self, ui: &mut Ui, e: &Evento, sit: Situacao, criador: bool, agora: i64) {
        let p = self.p;
        let hoje = historico::hoje();

        // título + selo da situação
        ui.horizontal(|ui| {
            let (rotulo, bg, fg) = match sit {
                Situacao::Andamento => ("AO VIVO", alfa(VERDE, 0.16), VERDE),
                Situacao::Futuro => ("EM BREVE", p.soon_bg, p.soon_ink),
                Situacao::Terminado => ("TERMINOU", p.chip, p.muted),
            };
            ui.allocate_ui_with_layout(
                vec2((ui.available_width() - 90.0).max(60.0), 24.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.add(egui::Label::new(txt(&e.nome, 17.0, Peso::Negrito, p.text)).truncate());
                },
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let g = ui.painter().layout_no_wrap(rotulo.to_string(), fonte(11.0, Peso::Pesado), fg);
                let (r, _) = ui.allocate_exact_size(vec2(g.size().x + 18.0, 22.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(11), bg);
                if sit == Situacao::Andamento {
                    // bolinha "ao vivo" piscando devagar
                    let a = 0.5 + 0.5 * ((self.an.agora * 3.0).sin() as f32);
                    ui.painter().circle_filled(r.left_center() + vec2(9.0, 0.0), 3.0, alfa(VERDE, a));
                    self.an.ativo = true;
                }
                let x = if sit == Situacao::Andamento { 4.0 } else { 0.0 };
                ui.painter().galley(r.center() - g.size() / 2.0 + vec2(x, 0.0), g, fg);
            });
        });

        // período, plataforma e o que conta
        ui.label(txt(e.rotulo_periodo(), 13.0, Peso::Semi, p.text));
        let plataforma = match e.plataforma() {
            PlataformaRanking::Todas => "Todas as plataformas".to_string(),
            pl => format!("Só {}", if pl == PlataformaRanking::Psn { "PlayStation" } else { pl.nome() }),
        };
        let conta = if e.jogo_chave.is_empty() {
            "quem jogar mais (tempo total)".to_string()
        } else {
            format!("quem jogar mais {}", e.jogo_nome)
        };
        ui.label(txt(format!("{plataforma} · {conta}"), 12.5, Peso::Normal, p.muted));
        let quando = match sit {
            Situacao::Futuro => format!("Começa em {}", fmt_falta(e.inicio_ms - agora)),
            Situacao::Andamento if e.fim_ms > 0 => format!("Termina em {}", fmt_falta(e.fim_ms - agora)),
            Situacao::Andamento => "Sem data para acabar".to_string(),
            Situacao::Terminado => format!("Terminou {}", fmt_relativo(e.fim_ms)),
        };
        ui.label(txt(quando, 12.5, Peso::Semi, if sit == Situacao::Andamento { VERDE } else { p.muted }));

        if !e.descricao.trim().is_empty() {
            ui.add_space(2.0);
            ui.label(txt(e.descricao.trim(), 13.5, Peso::Normal, p.text));
        }

        // recompensas (texto do criador + molduras de cada colocação)
        let premios: Vec<(usize, Vec<&str>)> = (0..e.qtd_vencedores())
            .map(|i| (i, e.premios_de(i)))
            .filter(|(_, v)| !v.is_empty())
            .collect();
        if !e.recompensas.trim().is_empty() || !premios.is_empty() {
            ui.add_space(2.0);
            let t = self.an.agora;
            let mut animada = false;
            Frame::default()
                .fill(p.chip)
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                        desenho::icone(ui.painter(), Icone::Trofeu, r.center(), 18.0, p.gold, 0.0);
                        ui.label(txt("Recompensas", 13.5, Peso::Negrito, p.text));
                    });
                    if !e.recompensas.trim().is_empty() {
                        ui.label(txt(e.recompensas.trim(), 13.0, Peso::Normal, p.text));
                    }
                    for (i, ms) in &premios {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for m in ms {
                                let (r, _) = ui.allocate_exact_size(vec2(32.0, 34.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 10.0, p.card);
                                animada |= desenho::moldura(ui.painter(), m, r.center(), 10.0, t, 1.0);
                            }
                            ui.add_space(4.0);
                            let nomes: Vec<&str> = ms.iter().map(|m| crate::molduras::nome(m)).collect();
                            let texto = if ms.len() == 1 {
                                format!("{}º lugar: moldura {}", i + 1, nomes[0])
                            } else {
                                format!("{}º lugar: molduras {}", i + 1, nomes.join(" + "))
                            };
                            ui.add(egui::Label::new(txt(texto, 13.0, Peso::Semi, p.text)).wrap());
                        });
                    }
                    if !premios.is_empty() {
                        ui.label(txt(
                            "As molduras são entregues sozinhas quando o evento acaba.",
                            11.5,
                            Peso::Normal,
                            p.muted,
                        ));
                    }
                });
            if animada {
                self.an.ativo = true;
            }
        }

        // quem está na frente / vencedores
        ui.add_space(4.0);
        let n = e.qtd_vencedores();
        match sit {
            Situacao::Futuro => {
                ui.label(txt(
                    "O placar aparece quando o evento começar.",
                    12.5,
                    Peso::Normal,
                    p.muted,
                ));
            }
            _ => {
                let titulo = if sit == Situacao::Terminado {
                    if n == 1 { "Vencedor" } else { "Vencedores" }
                } else {
                    "Na frente agora"
                };
                ui.label(txt(titulo, 14.0, Peso::Negrito, p.text));
                let itens = e.ranking(&self.hist, hoje);
                if itens.is_empty() {
                    let t = if self.hist_quando.is_none() {
                        "Carregando..."
                    } else {
                        "Ninguém jogou ainda."
                    };
                    ui.label(txt(t, 13.0, Peso::Normal, p.muted));
                }
                for (pos, it) in itens.iter().take(n).enumerate() {
                    self.linha_top(ui, pos, it, e.jogo_chave.is_empty());
                }
            }
        }

        if criador {
            ui.add_space(4.0);
            let editando = self.ev_premios_edit.as_ref().is_some_and(|(id, _)| *id == e.id);
            if editando {
                self.editor_premios(ui, e);
            }
            let mut editar = false;
            let mut apagar = false;
            let mut premios = false;
            ui.columns(3, |c| {
                editar = botao(&mut c[0], &mut self.an, &p, &format!("ev_ed_{}", e.id), "Editar", false).clicked();
                premios = botao(&mut c[1], &mut self.an, &p, &format!("ev_pr_{}", e.id), "Recompensas", false).clicked();
                apagar = botao(&mut c[2], &mut self.an, &p, &format!("ev_ap_{}", e.id), "Apagar", false).clicked();
            });
            if premios {
                self.ev_premios_edit = if editando { None } else { Some((e.id.clone(), e.premios.clone())) };
            }
            if editar {
                let ev = serde_json::to_string(e).unwrap_or_default();
                let jogos = self.jogos_json();
                self.plat.evento_editar(&ev, &jogos);
            }
            if apagar {
                self.plat.evento_apagar(&e.id);
            }
        }
    }
}
