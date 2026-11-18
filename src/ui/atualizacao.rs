//! Aviso de versão nova (painel que sobe de baixo) e a parte "Atualizações" das configurações.

use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui::{self, pos2, vec2, Align, CornerRadius, Id, Layout, Order, Rect, Sense, Ui, UiBuilder};

use super::desenho;
use super::telas::{botao, grupo};
use super::tema::{alfa, fonte, txt, Peso};
use super::*;
use crate::atualizacao::{self, Versao, INTERVALO_MS, REPO};
use crate::plataforma::agora_ms;

impl AbyssHubApp {
    /// Procura versão nova (numa thread). `manual` = a pessoa tocou no botão.
    pub(super) fn procurar_atualizacao(&mut self, manual: bool) {
        if self.rx_upd.is_some() {
            return;
        }
        self.upd_manual = manual;
        let plat = self.plat.clone();
        let (tx, rx) = mpsc::channel();
        self.rx_upd = Some(rx);
        std::thread::spawn(move || {
            let r = plat
                .verificar_atualizacao(REPO)
                .and_then(|j| j.map(|j| atualizacao::ler_versao(&j)).transpose());
            let _ = tx.send(r);
        });
    }

    /// Chamado todo quadro.
    pub(super) fn cuidar_atualizacao(&mut self) {
        // resposta da busca
        if let Some(rx) = &self.rx_upd {
            match rx.try_recv() {
                Ok(r) => {
                    self.rx_upd = None;
                    let manual = std::mem::take(&mut self.upd_manual);
                    match r {
                        Ok(v) => {
                            let tem_versao = v.is_some();
                            self.dados.atualizacao.ultima = v;
                            self.dados.atualizacao.ultima_verificacao = agora_ms();
                            let nova = self.dados.atualizacao.mais_nova(VERSAO).cloned();
                            match nova {
                                Some(n) if manual => {
                                    self.ajustes_aberto = false;
                                    self.upd_aberto = true;
                                    self.dados.atualizacao.ofertada = n.tag.clone();
                                }
                                Some(n) => {
                                    let s = &self.dados.atualizacao;
                                    if s.dispensada != n.tag && s.ofertada != n.tag {
                                        self.dados.atualizacao.ofertada = n.tag.clone();
                                        self.upd_aberto = true;
                                    }
                                }
                                None if manual && tem_versao => {
                                    self.mostrar_toast("Você já está na versão mais recente", false)
                                }
                                None if manual => self.mostrar_toast("Ainda não há versões publicadas", false),
                                None => {}
                            }
                            self.salvar();
                        }
                        Err(e) => {
                            if manual {
                                let msg = if e.contains("Android") {
                                    e
                                } else {
                                    "Não foi possível verificar agora (sem internet?)".to_string()
                                };
                                self.mostrar_toast(msg, true);
                            }
                        }
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.rx_upd = None,
            }
        }

        // Procura sozinho: toda vez que o app abre (alguns segundos depois) e,
        // com ele aberto, a cada 6 horas. Se der erro (sem internet), tenta de novo em 10 minutos.
        let aberto_ha = self.boot.map_or(0.0, |b| self.an.agora - b);
        let pode_tentar = self
            .ultima_tentativa_upd
            .map_or(true, |t| t.elapsed() > Duration::from_secs(600));
        let ao_abrir = !self.upd_tentou && aberto_ha > 3.0;
        let passou_6h = agora_ms() - self.dados.atualizacao.ultima_verificacao > INTERVALO_MS;
        if self.rx_upd.is_none() && (ao_abrir || (passou_6h && pode_tentar)) {
            self.upd_tentou = true;
            self.ultima_tentativa_upd = Some(Instant::now());
            self.procurar_atualizacao(false);
        }

        // andamento do download
        let ativo = matches!(
            self.upd_baixa.estado.as_str(),
            "baixando" | "baixado" | "permissao" | "instalando"
        );
        if (self.upd_aberto || ativo)
            && self
                .ultimo_poll_upd
                .map_or(true, |t| t.elapsed() > Duration::from_millis(250))
        {
            let j = self.plat.estado_atualizacao();
            if !j.is_empty() {
                self.upd_baixa = atualizacao::ler_baixa(&j);
            }
            self.ultimo_poll_upd = Some(Instant::now());
            if ativo {
                self.an.ativo = true;
            }
        }
    }

    /// O painel "Nova versão disponível".
    pub(super) fn folha_atualizacao(&mut self, ctx: &egui::Context) {
        let prog = self.an.ir("folha_upd", if self.upd_aberto { 1.0 } else { 0.0 }, 0.34);
        if prog <= 0.002 {
            return;
        }
        let Some(nova) = self.dados.atualizacao.mais_nova(VERSAO).cloned() else {
            self.upd_aberto = false;
            return;
        };
        let tela = ctx.screen_rect();
        let p = self.p;
        let baixando = self.upd_baixa.estado == "baixando";
        egui::Area::new(Id::new("folha_upd"))
            .order(Order::Foreground)
            .fixed_pos(tela.min)
            .show(ctx, |ui| {
                ui.set_min_size(tela.size());
                let fundo = ui.interact(tela, Id::new("folha_upd_fundo"), Sense::click());
                ui.painter()
                    .rect_filled(tela, CornerRadius::ZERO, alfa(p.overlay, prog));
                let altura = (tela.height() * 0.62).max(360.0).min(tela.height() - 40.0);
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
                let _ = ui.interact(folha, Id::new("folha_upd_corpo"), Sense::click());
                ui.painter().rect_filled(
                    Rect::from_center_size(pos2(folha.center().x, folha.top() + 12.0), vec2(40.0, 5.0)),
                    CornerRadius::same(3),
                    p.line,
                );
                let mut area = folha.shrink2(vec2(18.0, 0.0));
                area.min.y += 28.0;
                ui.scope_builder(UiBuilder::new().max_rect(area), |ui| {
                    self.conteudo_atualizacao(ui, &nova)
                });
                if fundo.clicked() && !baixando {
                    self.upd_aberto = false;
                }
            });
    }

    fn conteudo_atualizacao(&mut self, ui: &mut Ui, nova: &Versao) {
        let p = self.p;
        ui.spacing_mut().item_spacing.y = 12.0;
        ui.label(txt("Nova versão disponível", 20.0, Peso::Negrito, p.text));

        // "Você tem 1.0.0 → 1.1.0"
        ui.horizontal(|ui| {
            ui.label(txt("Você tem", 14.0, Peso::Normal, p.muted));
            ui.label(txt(VERSAO, 14.0, Peso::Negrito, p.text));
            ui.label(txt("→", 14.0, Peso::Normal, p.muted));
            let tag = atualizacao::sem_v(&nova.tag).to_string();
            let g = ui.painter().layout_no_wrap(tag, fonte(13.5, Peso::Negrito), p.gold_ink);
            let (r, _) = ui.allocate_exact_size(g.size() + vec2(20.0, 10.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::same(12), p.gold);
            ui.painter().galley(r.min + vec2(10.0, 5.0), g, p.gold_ink);
        });

        let notas = atualizacao::notas_limpas(&nova.notas);
        let rodape = 150.0;
        egui::ScrollArea::vertical()
            .id_salt("upd_notas")
            .max_height((ui.available_height() - rodape).max(60.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if notas.is_empty() {
                    ui.label(txt("Melhorias e correções.", 13.5, Peso::Normal, p.muted));
                } else {
                    ui.label(txt(notas, 13.5, Peso::Normal, p.muted));
                }
            });

        let b = self.upd_baixa.clone();
        if !nova.apk.is_empty() && b.estado != "baixando" {
            ui.label(txt(
                format!("Tamanho: {}", atualizacao::fmt_tamanho(nova.tamanho)),
                12.5,
                Peso::Normal,
                p.muted,
            ));
        }
        if matches!(b.estado.as_str(), "baixando" | "baixado" | "instalando" | "permissao") {
            let alvo = b.progresso as f32 / 100.0;
            let v = self.an.ir("upd_barra", alvo, 0.3);
            desenho::barra(ui, &p, v, p.gold);
        }
        let (msg, ruim) = match b.estado.as_str() {
            "baixando" => (format!("Baixando… {}%", b.progresso), false),
            "baixado" | "instalando" => ("Abrindo o instalador… confirme em \"Atualizar\".".to_string(), false),
            "permissao" => (
                "Permita \"Instalar apps desconhecidos\" para o AbyssHub e volte: a instalação continua sozinha."
                    .to_string(),
                false,
            ),
            "erro" => (format!("Falha no download: {}. Verifique a internet.", b.msg), true),
            _ if nova.apk.is_empty() => (
                "Esta versão ainda não tem o APK anexado. Tente de novo em alguns minutos.".to_string(),
                true,
            ),
            _ => (String::new(), false),
        };
        if !msg.is_empty() {
            ui.label(txt(msg, 13.0, Peso::Normal, if ruim { p.bad } else { p.text }));
        }

        let baixando = b.estado == "baixando";
        let instalar = matches!(b.estado.as_str(), "baixado" | "instalando" | "permissao");
        let rotulo = if baixando {
            "Baixando…".to_string()
        } else if instalar {
            "Instalar".to_string()
        } else if b.estado == "erro" {
            "Tentar de novo".to_string()
        } else {
            "Atualizar agora".to_string()
        };
        ui.columns(2, |c| {
            if botao(&mut c[0], &mut self.an, &p, "upd_depois", "Agora não", false).clicked() && !baixando {
                self.dados.atualizacao.dispensada = nova.tag.clone();
                self.upd_aberto = false;
                self.salvar();
                self.mostrar_toast("Tudo bem. Você pode atualizar depois em Configurações", false);
            }
            if botao(&mut c[1], &mut self.an, &p, "upd_ir", &rotulo, true).clicked() && !baixando {
                if instalar {
                    self.plat.instalar_atualizacao();
                } else if nova.apk.is_empty() {
                    self.plat.abrir_link(&nova.pagina);
                } else {
                    self.upd_baixa = atualizacao::Baixa {
                        estado: "baixando".into(),
                        ..Default::default()
                    };
                    self.plat.baixar_atualizacao(&nova.apk);
                }
            }
        });
    }

    /// Grupo "Atualizações" nas configurações.
    pub(super) fn grupo_atualizacoes(&mut self, ui: &mut Ui) {
        let p = self.p;
        let nova = self.dados.atualizacao.mais_nova(VERSAO).cloned();
        let procurando = self.rx_upd.is_some();
        let ultima = self.dados.atualizacao.ultima_verificacao;
        grupo(ui, &p, |ui| {
            ui.horizontal(|ui| {
                ui.label(txt("Atualizações", 15.0, Peso::Negrito, p.text));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(txt(format!("versão {VERSAO}"), 12.5, Peso::Normal, p.muted));
                });
            });
            let (linha, cor) = if procurando {
                ("Procurando versão nova…".to_string(), p.muted)
            } else if let Some(n) = &nova {
                (format!("Versão {} disponível!", atualizacao::sem_v(&n.tag)), p.gold)
            } else if ultima > 0 {
                (
                    format!("Você está na versão mais recente · verificado {}", fmt_relativo(ultima)),
                    p.muted,
                )
            } else {
                ("O app procura versões novas sozinho.".to_string(), p.muted)
            };
            ui.label(txt(linha, 13.0, Peso::Normal, cor));
            match nova {
                Some(n) => {
                    let r = botao(
                        ui,
                        &mut self.an,
                        &p,
                        "upd_cfg",
                        &format!("Atualizar para {}", atualizacao::sem_v(&n.tag)),
                        true,
                    );
                    if r.clicked() {
                        self.ajustes_aberto = false;
                        self.upd_aberto = true;
                    }
                }
                None => {
                    if botao(ui, &mut self.an, &p, "upd_buscar", "Procurar atualização", false).clicked() && !procurando
                    {
                        self.procurar_atualizacao(true);
                    }
                }
            }
        });
    }
}
