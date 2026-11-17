//! As telas do AbyssHub: Início, Jogos, Histórico, detalhe de um jogo,
//! configurações (painel que sobe de baixo), barra de abas, botão dourado e aviso.

use std::f32::consts::TAU;

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CornerRadius, Frame, Id, Layout, Margin, Order, Rect, Sense, Shape,
    Stroke, Ui, UiBuilder,
};

use super::anim::vai_e_volta;
use super::desenho::{self, Icone};
use super::tema::{alfa, fonte, misturar_cor, txt, Peso};
use super::*;
use crate::plataforma::agora_ms;
use crate::sessoes::Sessao;

/// Faz o conteúdo "entrar": desliza (dx, dy) e aparece aos poucos. `p` vai de 0 a 1.
pub(super) fn entrada<R>(ui: &mut Ui, p: f32, dx: f32, dy: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let ox = (dx * (1.0 - p)).round().clamp(-120.0, 120.0) as i8;
    let oy = (dy * (1.0 - p)).round().clamp(-120.0, 120.0) as i8;
    Frame::default()
        .outer_margin(Margin {
            left: ox,
            right: -ox,
            top: oy,
            bottom: -oy,
        })
        .show(ui, |ui| {
            ui.multiply_opacity(p.clamp(0.0, 1.0));
            add(ui)
        })
        .inner
}

impl AbyssHubApp {
    // =====================================================================
    // Estrutura da página
    // =====================================================================

    pub(super) fn pagina(&mut self, ui: &mut Ui, topo: f32) {
        if !self.permissao {
            egui::ScrollArea::vertical()
                .id_salt("permissao")
                .show(ui, |ui| self.tela_permissao(ui, topo));
            return;
        }
        let chave = match &self.detalhe {
            Some(p) => format!("det:{p}"),
            None => format!("{:?}", self.aba),
        };
        let p = self.an.desde(self.inicio_pagina, 0.0, 0.38);
        let dx = self.direcao * 28.0;
        egui::ScrollArea::vertical()
            .id_salt(chave)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                entrada(ui, p, dx, 0.0, |ui| match self.detalhe.clone() {
                    Some(pacote) => self.tela_detalhe(ui, topo, &pacote),
                    None => match self.aba {
                        Aba::Inicio => self.tela_inicio(ui, topo),
                        Aba::Jogos => self.tela_jogos(ui, topo),
                        Aba::Leaderboard => self.tela_leaderboard(ui, topo),
                        Aba::Historico => self.tela_historico(ui, topo),
                    },
                });
            });
    }

    /// Área dos cartões, com as margens do AbyssSub e espaço para o botão dourado.
    pub(super) fn conteudo(&mut self, ui: &mut Ui, add: impl FnOnce(&mut Self, &mut Ui)) {
        Frame::default()
            .inner_margin(Margin {
                left: 16,
                right: 16,
                top: 18,
                bottom: 100,
            })
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 14.0;
                add(self, ui);
            });
    }

    /// Cartão branco/azul-escuro arredondado que entra subindo (fadeUp).
    pub(super) fn cartao(&mut self, ui: &mut Ui, i: usize, add: impl FnOnce(&mut Self, &mut Ui)) -> egui::Response {
        let primeira = self.boot.map_or(false, |b| (self.inicio_pagina - b).abs() < 0.01);
        let atraso = if primeira {
            0.15 + i as f32 * 0.08
        } else {
            0.04 + i as f32 * 0.05
        };
        let p = self.an.desde(self.inicio_pagina, atraso, 0.55);
        let (fundo, borda) = (self.p.card, alfa(self.p.brand, 0.10 * self.escuro));
        let sombra = alfa(Color32::from_rgb(14, 28, 74), 0.07 * (1.0 - self.escuro));
        entrada(ui, p, 0.0, 14.0, |ui| {
            Frame::default()
                .fill(fundo)
                .stroke(Stroke::new(1.0_f32, borda))
                .shadow(egui::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 0,
                    color: sombra,
                })
                .corner_radius(CornerRadius::same(18))
                .inner_margin(Margin::same(16))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 8.0;
                    add(self, ui);
                })
                .response
        })
    }

    pub(super) fn titulo_cartao(&self, ui: &mut Ui, titulo: &str, dica: &str) {
        ui.horizontal(|ui| {
            ui.label(txt(titulo, 17.0, Peso::Negrito, self.p.text));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(txt(dica, 12.5, Peso::Normal, self.p.muted));
            });
        });
        ui.add_space(4.0);
    }

    pub(super) fn vazio(&self, ui: &mut Ui, titulo: &str, texto: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            desenho::icone(
                ui.painter(),
                Icone::Controle,
                ui.cursor().min + vec2(ui.available_width() / 2.0, 14.0),
                32.0,
                self.p.muted,
                0.0,
            );
            ui.add_space(34.0);
            ui.label(txt(titulo, 15.0, Peso::Semi, self.p.text));
            ui.label(txt(texto, 13.5, Peso::Normal, self.p.muted));
            ui.add_space(6.0);
        });
    }

    // =====================================================================
    // Topo em degradê (o "hero" do AbyssSub)
    // =====================================================================

    pub(super) fn hero(&mut self, ui: &mut Ui, topo: f32, add: impl FnOnce(&mut Self, &mut Ui)) {
        let fundo = ui.painter().add(Shape::Noop);
        let resp = Frame::default()
            .inner_margin(Margin {
                left: 16,
                right: 16,
                top: (16.0 + topo).min(120.0) as i8,
                bottom: 22,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 6.0;
                add(self, ui);
            })
            .response;
        let r = resp.rect;

        // brilho que flutua devagar (9 s indo, 9 s voltando)
        let e = if self.dados.animacoes {
            let t = (self.an.agora / 9.0) % 2.0;
            vai_e_volta(if t < 1.0 { t as f32 } else { 2.0 - t as f32 })
        } else {
            0.0
        };
        let centro = pos2(r.right() - 60.0 - 30.0 * e, r.top() + topo + 40.0 + 25.0 * e);
        let formas = vec![
            desenho::degrade_topo(r, 26.0, self.p.hero_a, self.p.hero_b),
            desenho::brilho(centro, 130.0 * (1.0 + 0.15 * e), alfa(Color32::WHITE, 0.13)),
        ];
        ui.painter().set(fundo, Shape::Vec(formas));
    }

    /// Logo + nome + engrenagem. `titulo` = None mostra "AbyssHub".
    pub(super) fn barra_marca(&mut self, ui: &mut Ui, titulo: Option<&str>) {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
            if let Some(logo) = &self.logo {
                ui.painter().rect_filled(
                    r.translate(vec2(0.0, 2.0)),
                    CornerRadius::same(10),
                    alfa(Color32::BLACK, 0.25),
                );
                ui.painter().image(
                    logo.id(),
                    r,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            ui.add_space(2.0);
            match titulo {
                None => {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(txt("Abyss", 19.0, Peso::Negrito, self.p.on_hero));
                    ui.label(txt("Hub", 19.0, Peso::Negrito, self.p.gold));
                }
                Some(t) => {
                    ui.label(txt(t, 19.0, Peso::Negrito, self.p.on_hero));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if desenho::botao_icone(ui, &mut self.an, &self.p, "config", Icone::Engrenagem).clicked() {
                    self.ajustes_aberto = true;
                }
            });
        });
        ui.add_space(10.0);
    }

    pub(super) fn seletor_periodo(&mut self, ui: &mut Ui) {
        let mut i = self.periodo.indice();
        if desenho::seletor(ui, &mut self.an, &self.p, "periodo", &Periodo::NOMES, &mut i, true) {
            self.mudar_periodo(Periodo::de(i));
        }
        ui.add_space(12.0);
    }

    pub(super) fn rotulo_hero(&self, ui: &mut Ui, texto: &str) {
        ui.label(txt(texto.to_uppercase(), 12.0, Peso::Negrito, alfa(self.p.on_hero, 0.75)).extra_letter_spacing(1.2));
    }

    /// O número grande do topo, contando até o valor (como o total do AbyssSub).
    pub(super) fn total_hero(&mut self, ui: &mut Ui, chave: &str, ms: i64) {
        let alvo = (ms / 1000) as f32;
        let v = self.an.ir_de(&format!("total:{chave}"), 0.0, alvo, 0.8);
        ui.label(txt(fmt_duracao((v as i64) * 1000), 40.0, Peso::Pesado, self.p.on_hero));
    }

    pub(super) fn sub_hero(&self, ui: &mut Ui, texto: &str) {
        ui.label(txt(texto, 13.0, Peso::Normal, alfa(self.p.on_hero, 0.8)));
    }

    /// As duas "pílulas" translúcidas embaixo do total.
    pub(super) fn pilulas(&self, ui: &mut Ui, itens: [(&str, String); 2]) {
        ui.add_space(10.0);
        let largura = (ui.available_width() - 10.0) / 2.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for (rotulo, valor) in itens {
                let (r, _) = ui.allocate_exact_size(vec2(largura, 60.0), Sense::hover());
                let pt = ui.painter();
                pt.rect_filled(r, CornerRadius::same(14), alfa(Color32::WHITE, 0.12));
                pt.rect_filled(r.shrink(1.0), CornerRadius::same(13), alfa(self.p.hero_b, 0.55));
                pt.rect_filled(r.shrink(1.0), CornerRadius::same(13), alfa(Color32::WHITE, 0.08));
                let dentro = pt.with_clip_rect(r.shrink(10.0));
                dentro.text(
                    r.left_top() + vec2(12.0, 11.0),
                    Align2::LEFT_TOP,
                    rotulo,
                    fonte(12.0, Peso::Normal),
                    alfa(self.p.on_hero, 0.8),
                );
                dentro.text(
                    r.left_top() + vec2(12.0, 29.0),
                    Align2::LEFT_TOP,
                    valor,
                    fonte(17.0, Peso::Negrito),
                    self.p.on_hero,
                );
            }
        });
    }

    /// Selo "Jogando agora" com a bolinha pulsando.
    pub(super) fn jogando_agora(&mut self, ui: &mut Ui) {
        let Some((p, ini)) = self.aberta.clone() else { return };
        if !self.visivel(&p) {
            return;
        }
        let nome = self.nome(&p);
        ui.add_space(6.0);
        let texto = format!("Jogando agora: {nome} · {}", fmt_duracao(agora_ms() - ini));
        let galley = ui
            .painter()
            .layout_no_wrap(texto, fonte(12.5, Peso::Semi), self.p.gold_ink);
        let (r, _) = ui.allocate_exact_size(vec2(galley.size().x + 34.0, 26.0), Sense::hover());
        let pt = ui.painter();
        pt.rect_filled(r, CornerRadius::same(13), self.p.gold);
        let pulso = if self.dados.animacoes {
            ((self.an.agora * 3.0).sin() as f32 + 1.0) / 2.0
        } else {
            0.5
        };
        let c = pos2(r.left() + 14.0, r.center().y);
        pt.circle_filled(c, 4.0 + 3.0 * pulso, alfa(self.p.gold_ink, 0.25 * (1.0 - pulso)));
        pt.circle_filled(c, 4.0, self.p.gold_ink);
        pt.galley(
            pos2(r.left() + 24.0, r.center().y - galley.size().y / 2.0),
            galley,
            self.p.gold_ink,
        );
    }

    // =====================================================================
    // Início
    // =====================================================================

    pub(super) fn tela_inicio(&mut self, ui: &mut Ui, topo: f32) {
        let agora = agora_ms();
        let ini = self.periodo.inicio();
        let linhas = self.totais(ini, agora);
        let total: i64 = linhas.iter().map(|l| l.tempo).sum();
        let sessoes: usize = linhas.iter().map(|l| l.sessoes).sum();

        // comparação com o período anterior do mesmo tamanho
        let (pilula1, pilula2) = if self.periodo == Periodo::Tudo {
            let media = if sessoes > 0 { total / sessoes as i64 } else { 0 };
            (
                ("Sessões", sessoes.to_string()),
                ("Média por sessão", fmt_duracao(media)),
            )
        } else {
            let tam = agora - ini;
            let antes: i64 = self.totais(ini - tam, ini).iter().map(|l| l.tempo).sum();
            let comp = if antes > 0 {
                let pct = ((total - antes) as f64 / antes as f64 * 100.0).round() as i64;
                if pct >= 0 {
                    format!("+{pct}%")
                } else {
                    format!("−{}%", -pct)
                }
            } else {
                "—".to_string()
            };
            let media = total / self.periodo.dias().max(1);
            (("vs. período anterior", comp), ("Média por dia", fmt_duracao(media)))
        };

        self.hero(ui, topo, |s, ui| {
            s.barra_marca(ui, None);
            s.seletor_periodo(ui);
            s.rotulo_hero(ui, &format!("Tempo de jogo · {}", s.periodo.nome()));
            s.total_hero(ui, "inicio", total);
            let n = linhas.len();
            s.sub_hero(
                ui,
                &format!(
                    "{n} {} · {sessoes} {}",
                    if n == 1 { "jogo" } else { "jogos" },
                    if sessoes == 1 { "sessão" } else { "sessões" }
                ),
            );
            s.jogando_agora(ui);
            s.pilulas(ui, [pilula1, pilula2]);
        });

        self.conteudo(ui, |s, ui| {
            // --- Por jogo (rosca) ---
            s.cartao(ui, 0, |s, ui| {
                let n = linhas.len();
                s.titulo_cartao(
                    ui,
                    "Por jogo",
                    &format!("{n} {}", if n == 1 { "jogo" } else { "jogos" }),
                );
                if linhas.is_empty() {
                    s.vazio(ui, "Nenhum jogo neste período", "Jogue alguma coisa e volte aqui.");
                } else {
                    s.rosca_por_jogo(ui, &linhas, total);
                }
            });

            // --- Últimos 7 dias ---
            s.cartao(ui, 1, |s, ui| {
                s.titulo_cartao(ui, "Últimos 7 dias", "toque num dia");
                let dias = Self::por_dia(&s.sessoes_visiveis());
                s.grafico_dias(ui, "inicio", &dias);
            });

            // --- Mais jogados ---
            if !linhas.is_empty() {
                s.cartao(ui, 2, |s, ui| {
                    ui.horizontal(|ui| {
                        ui.label(txt("Mais jogados", 17.0, Peso::Negrito, s.p.text));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let r = ui.add(
                                egui::Label::new(txt("ver todos ›", 13.0, Peso::Semi, s.p.brand)).sense(Sense::click()),
                            );
                            if r.clicked() {
                                s.ir_para_aba(Aba::Jogos);
                            }
                        });
                    });
                    let maior = linhas[0].tempo.max(1);
                    for (i, l) in linhas.iter().take(5).enumerate() {
                        let nome = s.nome(&l.pacote);
                        let sub = format!("último: {}", fmt_relativo(l.ultima));
                        if s.linha_jogo(
                            ui,
                            i,
                            &l.pacote,
                            &nome,
                            &sub,
                            l.tempo,
                            Some(l.tempo as f32 / maior as f32),
                            None,
                        ) {
                            s.abrir_detalhe(&l.pacote);
                        }
                    }
                });
            }
        });
    }

    pub(super) fn rosca_por_jogo(&mut self, ui: &mut Ui, linhas: &[Linha], total: i64) {
        let mut fatias: Vec<(String, i64, Color32)> = linhas
            .iter()
            .take(5)
            .enumerate()
            .map(|(i, l)| (self.nome(&l.pacote), l.tempo, cor_categoria(i)))
            .collect();
        let resto: i64 = linhas.iter().skip(5).map(|l| l.tempo).sum();
        if resto > 0 {
            fatias.push(("Outros".into(), resto, cor_categoria(6)));
        }

        let prog = self.an.desde(self.inicio_dados, 0.1, 0.9);
        let pop = self.an.desde(self.inicio_dados, 0.35, 0.5);
        ui.horizontal(|ui| {
            let tam = 150.0;
            let (r, _) = ui.allocate_exact_size(vec2(tam, tam), Sense::hover());
            let valores: Vec<(f32, Color32)> = fatias.iter().map(|f| (f.1 as f32, f.2)).collect();
            desenho::rosca(ui.painter(), r.center(), tam / 2.0, 22.0, &valores, prog, self.p.chip);
            let pt = ui.painter();
            let esc = 0.9 + 0.1 * pop;
            pt.text(
                r.center() - vec2(0.0, 10.0 * esc),
                Align2::CENTER_CENTER,
                "no período",
                fonte(12.0 * esc, Peso::Normal),
                alfa(self.p.muted, pop),
            );
            pt.text(
                r.center() + vec2(0.0, 9.0 * esc),
                Align2::CENTER_CENTER,
                fmt_duracao(total),
                fonte(17.0 * esc, Peso::Pesado),
                alfa(self.p.text, pop),
            );

            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.add_space(((tam - fatias.len() as f32 * 22.0) / 2.0).max(0.0));
                for (i, (nome, t, cor)) in fatias.iter().enumerate() {
                    let pin = self.an.desde(self.inicio_dados, 0.2 + i as f32 * 0.05, 0.4);
                    entrada(ui, pin, 10.0, 0.0, |ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
                        let pt = ui.painter();
                        pt.rect_filled(
                            Rect::from_center_size(pos2(r.left() + 6.0, r.center().y), vec2(11.0, 11.0)),
                            CornerRadius::same(3),
                            *cor,
                        );
                        let pct = (*t as f64 / total.max(1) as f64 * 100.0).round();
                        let direita = format!("{pct:.0}% · {}", fmt_curto(*t));
                        let g = pt.layout_no_wrap(direita, fonte(13.0, Peso::Normal), self.p.muted);
                        let xd = r.right() - g.size().x;
                        pt.galley(pos2(xd, r.center().y - g.size().y / 2.0), g, self.p.muted);
                        pt.with_clip_rect(Rect::from_min_max(r.min, pos2(xd - 6.0, r.max.y)))
                            .text(
                                pos2(r.left() + 18.0, r.center().y),
                                Align2::LEFT_CENTER,
                                nome,
                                fonte(13.5, Peso::Normal),
                                self.p.text,
                            );
                    });
                }
            });
        });
    }

    /// Gráfico de barras dos últimos 7 dias. As barras crescem ao aparecer.
    pub(super) fn grafico_dias(&mut self, ui: &mut Ui, chave: &str, dias: &[(i64, i64, usize)]) {
        let maior = dias.iter().map(|d| d.1).max().unwrap_or(0).max(1);
        let altura = 130.0;
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), altura + 40.0), Sense::hover());
        let n = dias.len().max(1);
        let w = r.width() / n as f32;
        let base = r.top() + 18.0 + altura;
        let selecionado = self.dia_sel.unwrap_or(n - 1).min(n - 1);

        for (i, (ini, t, _)) in dias.iter().enumerate() {
            let col = Rect::from_min_max(
                pos2(r.left() + w * i as f32, r.top()),
                pos2(r.left() + w * (i + 1) as f32, r.bottom()),
            );
            let resp = ui.interact(col, ui.id().with((chave, "dia", i)), Sense::click());
            if resp.clicked() {
                self.dia_sel = Some(i);
            }
            let cresce = self.an.desde(self.inicio_dados, 0.15 + i as f32 * 0.035, 0.7);
            let alvo_h = (altura * (*t as f32 / maior as f32)).max(4.0);
            let h = alvo_h * cresce;
            let x0 = col.left() + w * 0.12;
            let x1 = col.right() - w * 0.12;
            let barra = Rect::from_min_max(pos2(x0, base - h), pos2(x1, base));
            let eh_hoje = i == n - 1;
            let sel = self.an.ir(
                &format!("bar:{chave}:{i}"),
                if i == selecionado { 1.0 } else { 0.0 },
                0.25,
            );
            let base_cor = if eh_hoje { self.p.bar_now } else { self.p.bar };
            let cor = misturar_cor(
                base_cor,
                self.p.bar_up,
                if eh_hoje && self.dia_sel.is_none() { 0.0 } else { sel },
            );
            let pt = ui.painter();
            pt.rect_filled(
                barra,
                CornerRadius {
                    nw: 6,
                    ne: 6,
                    sw: 3,
                    se: 3,
                },
                cor,
            );
            if *t > 0 && sel > 0.01 {
                pt.text(
                    pos2(barra.center().x, base - h - 4.0),
                    Align2::CENTER_BOTTOM,
                    fmt_curto(*t),
                    fonte(11.5, Peso::Semi),
                    alfa(self.p.text, sel),
                );
            }
            pt.text(
                pos2(barra.center().x, base + 6.0),
                Align2::CENTER_TOP,
                if eh_hoje {
                    "hoje".to_string()
                } else {
                    fmt_dia_curto(*ini)
                },
                fonte(11.0, if i == selecionado { Peso::Semi } else { Peso::Normal }),
                if i == selecionado { self.p.text } else { self.p.muted },
            );
        }

        if let Some((ini, t, jogos)) = dias.get(selecionado) {
            let texto = if *t > 0 {
                format!(
                    "{} · {} em {jogos} {}",
                    fmt_dia(*ini),
                    fmt_duracao(*t),
                    if *jogos == 1 { "jogo" } else { "jogos" }
                )
            } else {
                format!("{} · nada jogado", fmt_dia(*ini))
            };
            ui.label(txt(texto, 12.5, Peso::Normal, self.p.muted));
        }
    }

    /// Uma linha de jogo (avatar, nome, detalhe, tempo). Devolve true se tocou.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn linha_jogo(
        &mut self,
        ui: &mut Ui,
        i: usize,
        pacote: &str,
        nome: &str,
        sub: &str,
        tempo: i64,
        fracao: Option<f32>,
        rank: Option<usize>,
    ) -> bool {
        let pin = self.an.desde(self.inicio_dados, 0.25 + i.min(15) as f32 * 0.035, 0.4);
        let id = ui.id().with(("linha", pacote));
        let p = self.p;
        let mut tocou = false;
        entrada(ui, pin, 0.0, 10.0, |ui| {
            let topo = ui.cursor().top();
            if i > 0 {
                let x = ui.max_rect().x_range();
                ui.painter().hline(x, topo, Stroke::new(1.0_f32, p.line));
            }
            let fundo = ui.painter().add(Shape::Noop);
            ui.add_space(10.0);
            let dentro = ui
                .horizontal(|ui| {
                    if let Some(rk) = rank {
                        let (r, _) = ui.allocate_exact_size(vec2(44.0, 30.0), Sense::hover());
                        let (bg, fg) = if rk == 0 {
                            (p.soon_bg, p.soon_ink)
                        } else {
                            (p.chip, p.text)
                        };
                        ui.painter().rect_filled(r, CornerRadius::same(10), bg);
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            format!("#{}", rk + 1),
                            fonte(13.0, Peso::Negrito),
                            fg,
                        );
                    }
                    self.icone_jogo(ui, pacote, nome, 40.0);
                    let largura_tempo = 86.0;
                    ui.allocate_ui_with_layout(
                        vec2((ui.available_width() - largura_tempo).max(40.0), 40.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.add(egui::Label::new(txt(nome, 16.0, Peso::Semi, p.text)).truncate());
                            ui.add(egui::Label::new(txt(sub, 12.0, Peso::Normal, p.muted)).truncate());
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(txt(fmt_duracao(tempo), 16.5, Peso::Pesado, p.text));
                    });
                })
                .response
                .rect;
            if let Some(f) = fracao {
                ui.add_space(2.0);
                let cresce = self.an.desde(self.inicio_dados, 0.3 + i.min(15) as f32 * 0.035, 0.7);
                desenho::barra(ui, &p, f * cresce, p.brand2);
            }
            ui.add_space(8.0);
            let area = Rect::from_min_max(
                pos2(dentro.left() - 6.0, topo + 2.0),
                pos2(dentro.right() + 6.0, ui.cursor().top() - 2.0),
            );
            let resp = ui.interact(area, id, Sense::click());
            let aperto = self.an.ir(
                &format!("press:{pacote}:{i}"),
                if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 },
                0.15,
            );
            if aperto > 0.01 {
                ui.painter().set(
                    fundo,
                    Shape::rect_filled(area, CornerRadius::same(12), alfa(p.chip, aperto)),
                );
            }
            tocou = resp.clicked();
        });
        tocou
    }

    // =====================================================================
    // Jogos (ranking)
    // =====================================================================

    pub(super) fn tela_jogos(&mut self, ui: &mut Ui, topo: f32) {
        let agora = agora_ms();
        let linhas = self.totais(self.periodo.inicio(), agora);
        let total: i64 = linhas.iter().map(|l| l.tempo).sum();
        let sessoes: usize = linhas.iter().map(|l| l.sessoes).sum();
        let top = linhas
            .first()
            .map(|l| self.nome(&l.pacote))
            .unwrap_or_else(|| "—".into());

        self.hero(ui, topo, |s, ui| {
            s.barra_marca(ui, Some("Jogos"));
            s.seletor_periodo(ui);
            s.rotulo_hero(ui, &format!("Ranking · {}", s.periodo.nome()));
            s.total_hero(ui, "jogos", total);
            let n = linhas.len();
            s.sub_hero(
                ui,
                &format!("{n} {} com tempo registrado", if n == 1 { "jogo" } else { "jogos" }),
            );
            s.pilulas(ui, [("Mais jogado", top), ("Sessões", sessoes.to_string())]);
        });

        self.conteudo(ui, |s, ui| {
            s.cartao(ui, 0, |s, ui| {
                s.titulo_cartao(ui, "Todos os jogos", "toque para ver mais");
                if linhas.is_empty() {
                    s.vazio(ui, "Nada por aqui ainda", "Os jogos aparecem quando você joga.");
                }
                let maior = linhas.first().map(|l| l.tempo).unwrap_or(1).max(1);
                for (i, l) in linhas.iter().enumerate() {
                    let nome = s.nome(&l.pacote);
                    let sub = format!(
                        "{} {} · último: {}",
                        l.sessoes,
                        if l.sessoes == 1 { "sessão" } else { "sessões" },
                        fmt_relativo(l.ultima)
                    );
                    if s.linha_jogo(
                        ui,
                        i,
                        &l.pacote,
                        &nome,
                        &sub,
                        l.tempo,
                        Some(l.tempo as f32 / maior as f32),
                        Some(i),
                    ) {
                        s.abrir_detalhe(&l.pacote);
                    }
                }
            });
            s.cartao(ui, 1, |s, ui| {
                ui.label(txt("Falta algum jogo?", 15.0, Peso::Semi, s.p.text));
                ui.label(txt(
                    "Os jogos são detectados sozinhos. Se algum não aparecer, marque ele como jogo nas configurações.",
                    13.0,
                    Peso::Normal,
                    s.p.muted,
                ));
                if botao(ui, &mut s.an, &s.p, "abrir_cfg", "Abrir configurações", false).clicked() {
                    s.ajustes_aberto = true;
                }
            });
        });
    }

    // =====================================================================
    // Histórico
    // =====================================================================

    pub(super) fn tela_historico(&mut self, ui: &mut Ui, topo: f32) {
        let mut sess = self.sessoes_visiveis();
        sess.sort_by(|a, b| b.inicio.cmp(&a.inicio));
        let ini7 = inicio_do_dia(6);
        let agora = agora_ms();
        let na_semana: Vec<&Sessao> = sess.iter().filter(|s| s.fim > ini7).collect();
        let total7: i64 = na_semana
            .iter()
            .map(|s| sessoes::tempo_no_intervalo(s, ini7, agora))
            .sum();
        let maior = na_semana.iter().map(|s| s.duracao()).max().unwrap_or(0);
        let dias_jogados = Self::por_dia(&sess).iter().filter(|d| d.1 > 0).count();

        self.hero(ui, topo, |s, ui| {
            s.barra_marca(ui, Some("Histórico"));
            s.rotulo_hero(ui, "Últimos 7 dias");
            s.total_hero(ui, "historico", total7);
            let k = na_semana.len();
            s.sub_hero(
                ui,
                &format!("{k} {} na semana", if k == 1 { "sessão" } else { "sessões" }),
            );
            s.pilulas(
                ui,
                [
                    ("Dias jogados", format!("{dias_jogados} de 7")),
                    ("Maior sessão", fmt_duracao(maior)),
                ],
            );
        });

        // agrupa por dia (últimos 30 dias)
        let limite = inicio_do_dia(29);
        let mut grupos: Vec<(i64, Vec<Sessao>)> = Vec::new();
        for s in sess.into_iter().filter(|s| s.fim > limite) {
            let dia = (0..30).map(inicio_do_dia).find(|d| s.inicio >= *d).unwrap_or(limite);
            match grupos.last_mut() {
                Some((d, v)) if *d == dia => v.push(s),
                _ => grupos.push((dia, vec![s])),
            }
        }

        self.conteudo(ui, |s, ui| {
            if grupos.is_empty() {
                s.cartao(ui, 0, |s, ui| {
                    s.vazio(ui, "Sem sessões ainda", "Cada vez que você jogar, aparece aqui.");
                });
            }
            for (gi, (dia, lista)) in grupos.iter().enumerate() {
                let total_dia: i64 = lista.iter().map(|x| x.duracao()).sum();
                s.cartao(ui, gi.min(6), |s, ui| {
                    s.titulo_cartao(ui, &fmt_dia(*dia), &fmt_duracao(total_dia));
                    for (i, se) in lista.iter().enumerate() {
                        s.linha_sessao(ui, i, se, true);
                    }
                });
            }
        });
    }

    /// Linha de sessão: [14:03] Nome / até 15:10 ...... 1h 07m
    pub(super) fn linha_sessao(&mut self, ui: &mut Ui, i: usize, se: &Sessao, com_nome: bool) {
        let pin = self.an.desde(self.inicio_dados, 0.2 + i.min(15) as f32 * 0.03, 0.4);
        let p = self.p;
        let nome = self.nome(&se.pacote);
        let aberta = self
            .aberta
            .as_ref()
            .map_or(false, |a| a.0 == se.pacote && a.1 == se.inicio);
        let mut abrir = false;
        entrada(ui, pin, 0.0, 8.0, |ui| {
            let topo = ui.cursor().top();
            if i > 0 {
                ui.painter()
                    .hline(ui.max_rect().x_range(), topo, Stroke::new(1.0_f32, p.line));
            }
            ui.add_space(8.0);
            let r = ui
                .horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(64.0, 30.0), Sense::hover());
                    let (bg, fg) = if aberta {
                        (p.soon_bg, p.soon_ink)
                    } else {
                        (p.chip, p.text)
                    };
                    ui.painter().rect_filled(r, CornerRadius::same(10), bg);
                    ui.painter().text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        fmt_hora(se.inicio),
                        fonte(12.5, Peso::Negrito),
                        fg,
                    );
                    ui.allocate_ui_with_layout(
                        vec2((ui.available_width() - 80.0).max(40.0), 38.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            let titulo = if com_nome { nome.clone() } else { fmt_dia(se.inicio) };
                            ui.add(egui::Label::new(txt(titulo, 15.5, Peso::Semi, p.text)).truncate());
                            let sub = if aberta {
                                "jogando agora".to_string()
                            } else {
                                format!("até {}", fmt_hora(se.fim))
                            };
                            ui.label(txt(sub, 12.0, Peso::Normal, if aberta { p.gold } else { p.muted }));
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(txt(fmt_duracao(se.duracao()), 15.5, Peso::Pesado, p.text));
                    });
                })
                .response
                .rect;
            ui.add_space(6.0);
            if com_nome {
                let resp = ui.interact(r, ui.id().with(("sessao", &se.pacote, se.inicio)), Sense::click());
                abrir = resp.clicked();
            }
        });
        if abrir {
            self.abrir_detalhe(&se.pacote);
        }
    }

    // =====================================================================
    // Detalhe de um jogo
    // =====================================================================

    pub(super) fn tela_detalhe(&mut self, ui: &mut Ui, topo: f32, pacote: &str) {
        let nome = self.nome(pacote);
        let mut sess = self.sessoes_de(pacote);
        sess.sort_by(|a, b| b.inicio.cmp(&a.inicio));
        let agora = agora_ms();
        let ini = self.periodo.inicio();
        let no_periodo: Vec<&Sessao> = sess.iter().filter(|s| s.fim > ini).collect();
        let total: i64 = no_periodo
            .iter()
            .map(|s| sessoes::tempo_no_intervalo(s, ini, agora))
            .sum();
        let k = no_periodo.len();
        let media = if k > 0 { total / k as i64 } else { 0 };
        let maior = no_periodo.iter().map(|s| s.duracao()).max().unwrap_or(0);
        let ultima = sess.first().map(|s| s.fim).unwrap_or(0);

        self.hero(ui, topo, |s, ui| {
            ui.horizontal(|ui| {
                if desenho::botao_icone(ui, &mut s.an, &s.p, "voltar", Icone::Voltar).clicked() {
                    s.fechar_detalhe();
                }
                ui.add_space(2.0);
                s.icone_jogo(ui, pacote, &nome, 36.0);
                ui.add(egui::Label::new(txt(&nome, 19.0, Peso::Negrito, s.p.on_hero)).truncate());
            });
            ui.add_space(10.0);
            s.seletor_periodo(ui);
            s.rotulo_hero(ui, &format!("Tempo · {}", s.periodo.nome()));
            s.total_hero(ui, &format!("det:{pacote}"), total);
            s.sub_hero(
                ui,
                &format!(
                    "{k} {} · último: {}",
                    if k == 1 { "sessão" } else { "sessões" },
                    fmt_relativo(ultima)
                ),
            );
            s.jogando_agora(ui);
            s.pilulas(
                ui,
                [
                    ("Média por sessão", fmt_duracao(media)),
                    ("Sessão mais longa", fmt_duracao(maior)),
                ],
            );
        });

        self.conteudo(ui, |s, ui| {
            s.cartao(ui, 0, |s, ui| {
                s.titulo_cartao(ui, "Últimos 7 dias", "toque num dia");
                let dias = Self::por_dia(&sess);
                s.grafico_dias(ui, pacote, &dias);
            });

            s.cartao(ui, 1, |s, ui| {
                let mut jogo = s.dados.eh_jogo(pacote);
                if linha_interruptor(
                    ui,
                    &mut s.an,
                    &s.p,
                    &format!("jogo:{pacote}"),
                    "Contar como jogo",
                    Some("Desligue se este app não for um jogo."),
                    &mut jogo,
                ) {
                    s.dados.definir_jogo(pacote, jogo);
                    s.salvar();
                }
            });

            s.cartao(ui, 2, |s, ui| {
                s.titulo_cartao(ui, "Sessões", &format!("{} no total", sess.len()));
                if sess.is_empty() {
                    s.vazio(ui, "Nenhuma sessão ainda", "");
                }
                for (i, se) in sess.iter().take(100).enumerate() {
                    s.linha_sessao(ui, i, se, false);
                }
            });
        });
    }

    // =====================================================================
    // Sem permissão
    // =====================================================================

    pub(super) fn tela_permissao(&mut self, ui: &mut Ui, topo: f32) {
        self.hero(ui, topo, |s, ui| {
            s.barra_marca(ui, None);
            s.rotulo_hero(ui, "Bem-vindo");
            ui.label(txt("Falta 1 passo", 36.0, Peso::Pesado, s.p.on_hero));
            s.sub_hero(ui, "Libere o acesso ao uso para o AbyssHub contar seu tempo de jogo.");
        });
        self.conteudo(ui, |s, ui| {
            s.cartao(ui, 0, |s, ui| {
                s.titulo_cartao(ui, "Como liberar", "leva 10 segundos");
                for (i, passo) in [
                    "Toque no botão abaixo",
                    "Encontre o AbyssHub na lista",
                    "Ative \"Permitir acesso ao uso\"",
                    "Volte para cá",
                ]
                .iter()
                .enumerate()
                {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 14.0, s.p.chip);
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            (i + 1).to_string(),
                            fonte(13.0, Peso::Negrito),
                            s.p.text,
                        );
                        ui.label(txt(*passo, 15.0, Peso::Normal, s.p.text));
                    });
                }
                ui.add_space(6.0);
                if botao(ui, &mut s.an, &s.p, "permissao", "Abrir configurações", true).clicked() {
                    s.plat.abrir_permissao();
                }
            });
            s.cartao(ui, 1, |s, ui| {
                ui.label(txt("Seus dados ficam no celular", 15.0, Peso::Semi, s.p.text));
                ui.label(txt(
                    "O AbyssHub só lê quando cada app foi aberto e fechado. Nada é enviado para a internet.",
                    13.0,
                    Peso::Normal,
                    s.p.muted,
                ));
            });
        });
    }

    // =====================================================================
    // Barra de abas, botão dourado, aviso e configurações
    // =====================================================================

    pub(super) fn barra_abas(&mut self, ui: &mut Ui) {
        let r = ui.max_rect();
        ui.painter()
            .hline(r.x_range(), r.top(), Stroke::new(1.0_f32, self.p.line));
        let abas = [
            (Aba::Inicio, "Início", Icone::Casa),
            (Aba::Jogos, "Jogos", Icone::Controle),
            (Aba::Leaderboard, "Leaderboard", Icone::Trofeu),
            (Aba::Historico, "Histórico", Icone::Calendario),
        ];
        let w = r.width() / abas.len() as f32;
        for (i, (aba, nome, ic)) in abas.into_iter().enumerate() {
            let cel = Rect::from_min_size(pos2(r.left() + w * i as f32, r.top() + 4.0), vec2(w, 60.0));
            let resp = ui.interact(cel, Id::new(("aba", i)), Sense::click());
            let on = self
                .an
                .ir(&format!("aba:{i}"), if self.aba == aba { 1.0 } else { 0.0 }, 0.3);
            let aperto = self.an.ir(
                &format!("abap:{i}"),
                if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 },
                0.2,
            );
            let pt = ui.painter();
            let c = pos2(cel.center().x, cel.top() + 20.0);
            let pilula = Rect::from_center_size(c, vec2(60.0, 32.0) * (1.0 - 0.08 * aperto));
            pt.rect_filled(pilula, CornerRadius::same(16), alfa(self.p.tab_on, on));
            let cor = misturar_cor(self.p.muted, self.p.text, on);
            desenho::icone(pt, ic, c, 22.0 * (1.0 - 0.08 * aperto), cor, 0.0);
            pt.text(
                pos2(c.x, cel.top() + 43.0),
                Align2::CENTER_CENTER,
                nome,
                fonte(12.5, if on > 0.5 { Peso::Negrito } else { Peso::Semi }),
                cor,
            );
            if resp.clicked() {
                self.ir_para_aba(aba);
            }
        }
    }

    pub(super) fn botao_flutuante(&mut self, ctx: &egui::Context, baixo: f32) {
        let tela = ctx.screen_rect();
        let raio = 30.0;
        let centro = pos2(tela.right() - 16.0 - raio, tela.bottom() - baixo - 66.0 - 16.0 - raio);
        let pop = self.boot.map_or(1.0, |b| self.an.desde(b, 0.45, 0.45));
        let sincronizando = self.rx.is_some();
        egui::Area::new(Id::new("botao_dourado"))
            .order(Order::Middle)
            .fixed_pos(centro - vec2(40.0, 40.0))
            .show(ctx, |ui| {
                let (r, resp) = ui.allocate_exact_size(vec2(80.0, 80.0), Sense::click());
                let c = r.center();
                let aperto = self
                    .an
                    .ir("fab", if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 }, 0.2);
                let rr = raio * (0.9 + 0.1 * pop) * (1.0 - 0.07 * aperto);
                let pt = ui.painter();
                desenho::sombra_circulo(pt, c, rr, alfa(self.p.gold, 0.5 * pop));
                pt.circle_filled(c, rr, alfa(self.p.gold, pop));
                let giro = if sincronizando {
                    (self.an.agora * 5.0 % TAU as f64) as f32
                } else {
                    self.an.ir("fab_giro", 0.0, 0.3)
                };
                desenho::icone(pt, Icone::Atualizar, c, 26.0, alfa(self.p.gold_ink, pop), giro);
                if resp.clicked() && !sincronizando {
                    self.sync_manual = true;
                    self.precisa_apps = true;
                    self.sincronizar();
                }
            });
        if sincronizando {
            self.an.ativo = true;
        }
    }

    pub(super) fn desenhar_toast(&mut self, ctx: &egui::Context, baixo: f32) {
        let Some((texto, t0, erro)) = self.toast.clone() else {
            return;
        };
        let vida = (self.an.agora - t0) as f32;
        if vida > 2.6 {
            self.toast = None;
            return;
        }
        self.an.ativo = true;
        let entra = super::anim::suave(vida / 0.35);
        let sai = 1.0 - super::anim::suave((vida - 2.2) / 0.35);
        let a = entra.min(sai);
        let tela = ctx.screen_rect();
        let pt = ctx.layer_painter(egui::LayerId::new(Order::Tooltip, Id::new("aviso")));
        let (fundo, tinta) = if erro {
            (self.p.bad, Color32::WHITE)
        } else {
            (self.p.toast_bg, self.p.toast_ink)
        };
        let g = pt.layout(texto, fonte(14.0, Peso::Semi), tinta, tela.width() - 80.0);
        let tam = g.size() + vec2(36.0, 22.0);
        let centro = pos2(
            tela.center().x,
            tela.bottom() - baixo - 66.0 - 30.0 - tam.y / 2.0 + 16.0 * (1.0 - a),
        );
        let r = Rect::from_center_size(centro, tam);
        pt.rect_filled(
            r.translate(vec2(0.0, 4.0)),
            CornerRadius::same(14),
            alfa(Color32::BLACK, 0.18 * a),
        );
        pt.rect_filled(r, CornerRadius::same(14), alfa(fundo, a));
        pt.galley(r.min + vec2(18.0, 11.0), g, alfa(tinta, a));
    }

    pub(super) fn folha_ajustes(&mut self, ctx: &egui::Context, topo: f32) {
        let prog = self.an.ir("folha", if self.ajustes_aberto { 1.0 } else { 0.0 }, 0.34);
        if prog <= 0.002 {
            return;
        }
        let tela = ctx.screen_rect();
        let p = self.p;
        egui::Area::new(Id::new("folha"))
            .order(Order::Foreground)
            .fixed_pos(tela.min)
            .show(ctx, |ui| {
                ui.set_min_size(tela.size());
                let fundo = ui.interact(tela, Id::new("folha_fundo"), Sense::click());
                ui.painter()
                    .rect_filled(tela, CornerRadius::ZERO, alfa(p.overlay, prog));

                let altura = (tela.height() - topo - 48.0).max(200.0);
                let folha = Rect::from_min_size(
                    pos2(tela.left(), tela.bottom() - altura * prog),
                    vec2(tela.width(), altura),
                );
                ui.painter().rect_filled(
                    folha,
                    CornerRadius {
                        nw: 22,
                        ne: 22,
                        sw: 0,
                        se: 0,
                    },
                    p.card,
                );
                let _corpo = ui.interact(folha, Id::new("folha_corpo"), Sense::click());
                ui.painter().rect_filled(
                    Rect::from_center_size(pos2(folha.center().x, folha.top() + 12.0), vec2(40.0, 5.0)),
                    CornerRadius::same(3),
                    p.line,
                );

                let mut area = folha.shrink2(vec2(16.0, 0.0));
                area.min.y += 26.0;
                ui.scope_builder(UiBuilder::new().max_rect(area), |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("folha_rolagem")
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.conteudo_ajustes(ui));
                });

                if fundo.clicked() {
                    self.ajustes_aberto = false;
                }
            });
    }

    pub(super) fn conteudo_ajustes(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.spacing_mut().item_spacing.y = 12.0;
        ui.horizontal(|ui| {
            ui.label(txt("Configurações", 20.0, Peso::Negrito, p.text));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let r = ui.add(egui::Label::new(txt("Fechar", 14.0, Peso::Semi, p.brand)).sense(Sense::click()));
                if r.clicked() {
                    self.ajustes_aberto = false;
                }
            });
        });

        grupo(ui, &p, |ui| {
            ui.label(txt("Conta e Leaderboard", 15.0, Peso::Negrito, p.text));
            match self.conta.estado {
                crate::plataforma::EstadoConta::Logado => {
                    let conta = self.conta.clone();
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                        let pt = ui.painter().clone();
                        let item = crate::ranking::ItemRanking {
                            uid: conta.uid.clone(),
                            nome: conta.nome.clone(),
                            foto: conta.foto.clone(),
                            ..Default::default()
                        };
                        match self.imagens.foto(&item.foto) {
                            Some(tex) => desenho::imagem_recortada(&pt, tex, r, 22.0, Color32::WHITE),
                            None => desenho::desenhar_avatar(&pt, &p, &item.nome, &item.uid, r, self.escuro, true),
                        }
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.add(egui::Label::new(txt(&conta.nome, 15.0, Peso::Semi, p.text)).truncate());
                            ui.label(txt("Aparecendo no Leaderboard", 12.5, Peso::Normal, p.muted));
                        });
                    });
                    if botao(ui, &mut self.an, &p, "sair", "Sair da conta", false).clicked() {
                        self.plat.sair();
                        self.ultimo_check_conta = None;
                        self.mostrar_toast("Você saiu da conta", false);
                    }
                    if botao(
                        ui,
                        &mut self.an,
                        &p,
                        "apagar_lb",
                        "Sair e tirar meus dados do Leaderboard",
                        false,
                    )
                    .clicked()
                        && self.rx_apagar.is_none()
                    {
                        let plat = self.plat.clone();
                        let (tx, rx) = std::sync::mpsc::channel();
                        self.rx_apagar = Some(rx);
                        std::thread::spawn(move || {
                            let r = plat.apagar_meus_dados();
                            plat.sair();
                            let _ = tx.send(r);
                        });
                    }
                }
                crate::plataforma::EstadoConta::Indisponivel => {
                    ui.label(txt(
                        "O login não está disponível nesta versão do app.",
                        13.0,
                        Peso::Normal,
                        p.muted,
                    ));
                }
                _ => {
                    ui.label(txt("Entre para aparecer no Leaderboard.", 13.0, Peso::Normal, p.muted));
                    self.botao_google(ui);
                    self.botao_email(ui);
                }
            }
        });

        grupo(ui, &p, |ui| {
            ui.label(txt("Aparência", 15.0, Peso::Negrito, p.text));
            ui.label(txt(
                "Siga o tema do celular ou escolha um.",
                13.0,
                Peso::Normal,
                p.muted,
            ));
            let mut t = self.dados.tema as usize;
            if desenho::seletor(
                ui,
                &mut self.an,
                &self.p,
                "tema",
                &["Sistema", "Claro", "Escuro"],
                &mut t,
                false,
            ) {
                self.dados.tema = t as u8;
                self.salvar();
            }
            let mut anim = self.dados.animacoes;
            if linha_interruptor(
                ui,
                &mut self.an,
                &p,
                "anim",
                "Animações",
                Some("Brilho do topo, cartões e gráficos animados."),
                &mut anim,
            ) {
                self.dados.animacoes = anim;
                self.salvar();
            }
        });

        grupo(ui, &p, |ui| {
            ui.label(txt("Exibição", 15.0, Peso::Negrito, p.text));
            let mut todos = self.dados.mostrar_todos;
            if linha_interruptor(
                ui,
                &mut self.an,
                &p,
                "todos",
                "Mostrar todos os apps",
                Some("Inclui apps que não são jogos nas listas."),
                &mut todos,
            ) {
                self.dados.mostrar_todos = todos;
                self.salvar();
                self.inicio_dados = self.an.agora;
            }
        });

        grupo(ui, &p, |ui| {
            ui.label(txt("O que conta como jogo", 15.0, Peso::Negrito, p.text));
            ui.label(txt(
                "Os jogos são detectados sozinhos. Ajuste aqui se precisar.",
                13.0,
                Peso::Normal,
                p.muted,
            ));
            let mut lista: Vec<(String, String)> = self
                .apps
                .values()
                .filter(|a| a.abrivel && a.pacote != self.proprio)
                .map(|a| (a.pacote.clone(), a.nome.clone()))
                .collect();
            lista.sort_by_key(|(_, n)| n.to_lowercase());
            if lista.is_empty() {
                ui.label(txt("Carregando lista de apps...", 13.0, Peso::Normal, p.muted));
            }
            // jogos primeiro
            lista.sort_by_key(|(pk, _)| !self.dados.eh_jogo(pk));
            for (pacote, nome) in &lista {
                let mut j = self.dados.eh_jogo(pacote);
                ui.horizontal(|ui| {
                    self.icone_jogo(ui, pacote, nome, 30.0);
                    ui.allocate_ui_with_layout(
                        vec2((ui.available_width() - 60.0).max(40.0), 30.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.add(egui::Label::new(txt(nome, 14.5, Peso::Normal, p.text)).truncate());
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if desenho::interruptor(ui, &mut self.an, &p, &format!("ap:{pacote}"), &mut j).changed() {
                            self.dados.definir_jogo(pacote, j);
                            self.salvar();
                        }
                    });
                });
            }
        });

        grupo(ui, &p, |ui| {
            ui.label(txt("Histórico", 15.0, Peso::Negrito, p.text));
            ui.label(txt(
                format!("{} sessões salvas no celular", self.dados.sessoes.len()),
                14.0,
                Peso::Normal,
                p.text,
            ));
            ui.label(txt(
                "O Android só guarda o histórico de uso por alguns dias. Abra o AbyssHub pelo menos uma vez por semana para nenhum tempo se perder.",
                12.5,
                Peso::Normal,
                p.muted,
            ));
            if botao(ui, &mut self.an, &p, "perm2", "Permissão de acesso ao uso", false).clicked() {
                self.plat.abrir_permissao();
            }
        });

        self.grupo_atualizacoes(ui);

        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
            if let Some(logo) = &self.logo {
                ui.painter().image(
                    logo.id(),
                    r,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(txt("AbyssHub", 15.0, Peso::Negrito, p.text));
                ui.label(txt(
                    format!("versão {VERSAO} · conjunto Abyss"),
                    12.5,
                    Peso::Normal,
                    p.muted,
                ));
            });
        });
        ui.add_space(30.0);
    }
}

// ---------- Peças reaproveitadas ----------

/// Caixa de grupo das configurações (`.set-group`).
pub(super) fn grupo(ui: &mut Ui, p: &super::tema::Paleta, add: impl FnOnce(&mut Ui)) {
    Frame::default()
        .fill(p.field)
        .stroke(Stroke::new(1.0_f32, p.line))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 8.0;
            add(ui);
        });
}

/// Linha com título, descrição e interruptor à direita. Devolve true se mudou.
pub(super) fn linha_interruptor(
    ui: &mut Ui,
    an: &mut super::anim::Animador,
    p: &super::tema::Paleta,
    chave: &str,
    titulo: &str,
    desc: Option<&str>,
    valor: &mut bool,
) -> bool {
    let mut mudou = false;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            vec2((ui.available_width() - 60.0).max(60.0), 36.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(txt(titulo, 15.0, Peso::Semi, p.text));
                if let Some(d) = desc {
                    ui.label(txt(d, 12.5, Peso::Normal, p.muted));
                }
            },
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            mudou = desenho::interruptor(ui, an, p, chave, valor).changed();
        });
    });
    mudou
}

/// Botão largo (`.btn.primary` ou `.btn.ghost`), que afunda um pouco ao tocar.
pub(super) fn botao(
    ui: &mut Ui,
    an: &mut super::anim::Animador,
    p: &super::tema::Paleta,
    chave: &str,
    texto: &str,
    primario: bool,
) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
    let aperto = an.ir(
        &format!("btn:{chave}"),
        if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 },
        0.15,
    );
    let rr = Rect::from_center_size(r.center(), r.size() * (1.0 - 0.02 * aperto));
    let (fundo, tinta) = if primario {
        (p.primario, p.primario_ink)
    } else {
        (p.chip, p.text)
    };
    let pt = ui.painter();
    pt.rect_filled(rr, CornerRadius::same(14), fundo);
    if aperto > 0.0 {
        pt.rect_filled(rr, CornerRadius::same(14), alfa(Color32::BLACK, 0.08 * aperto));
    }
    pt.text(
        rr.center(),
        Align2::CENTER_CENTER,
        texto,
        fonte(15.0, Peso::Negrito),
        tinta,
    );
    resp
}
