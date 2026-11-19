//! Aba Placar: os 3 primeiros de cada dia, semana (segunda a domingo) ou mês, do mais recente
//! para trás, carregando mais conforme a pessoa desce. "Tudo" mostra quem está na frente agora.
//!
//! Também cuida do histórico por dia: envia o seu e busca o de todo mundo (usado no Placar e nos
//! Eventos), e busca a lista de eventos.

use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use eframe::egui::{self, vec2, Align, Align2, Layout, Sense, Ui};

use super::desenho;
use super::leaderboard::{chave_dia, cor_medalha};
use super::telas::botao;
use super::tema::{alfa, fonte, txt, Peso};
use super::*;
use crate::historico::{self, fmt_dia, TipoPeriodo};
use crate::plataforma::{agora_ms, EstadoConta};
use crate::ranking::{self, ItemRanking, ModoRanking, PlataformaRanking};

/// Busca o histórico de todo mundo de novo a cada 5 minutos (com o Placar ou os Eventos abertos).
const INTERVALO_HIST: Duration = Duration::from_secs(300);
/// Envia o seu histórico no máximo a cada 5 minutos (e só se mudou).
const INTERVALO_PUB: Duration = Duration::from_secs(300);
const INTERVALO_EVENTOS: Duration = Duration::from_secs(120);
const INTERVALO_TUDO: Duration = Duration::from_secs(120);
/// Quantos períodos aparecem de cada vez ao descer.
const LOTE: usize = 6;

pub(super) const PERIODOS: [&str; 4] = ["Dia", "Semana", "Mês", "Tudo"];

fn tipo(i: usize) -> Option<TipoPeriodo> {
    match i {
        0 => Some(TipoPeriodo::Dia),
        1 => Some(TipoPeriodo::Semana),
        2 => Some(TipoPeriodo::Mes),
        _ => None,
    }
}

impl AbyssHubApp {
    // =====================================================================
    // Dados (chamado todo quadro)
    // =====================================================================

    pub(super) fn cuidar_placar_e_eventos(&mut self) {
        self.receber_placar();
        if self.conta.estado != EstadoConta::Logado {
            return;
        }
        let agora = Instant::now();

        // 1) envia o seu histórico por dia
        let vencido = self
            .hist_publicado
            .as_ref()
            .map_or(true, |(_, t)| t.elapsed() >= INTERVALO_PUB);
        if vencido && self.rx_hist_pub.is_none() {
            let sess = self.sessoes_publicacao();
            let json = historico::montar_historico(&sess, |p| self.nome(p), |p| self.plataformas_de(p), agora_ms());
            let mudou = self.hist_publicado.as_ref().map_or(true, |(j, _)| *j != json);
            self.hist_publicado = Some((json.clone(), agora));
            if mudou {
                let plat = self.plat.clone();
                let (tx, rx) = mpsc::channel();
                self.rx_hist_pub = Some(rx);
                std::thread::spawn(move || {
                    let _ = tx.send(plat.publicar_historico(&json));
                });
            }
        }

        // 2) histórico de todo mundo, com o Placar ou os Eventos abertos
        let na_aba = matches!(self.aba, Aba::Placar | Aba::Eventos) && self.detalhe.is_none();
        if na_aba && self.rx_hist.is_none() && self.hist_pedido.map_or(true, |t| t.elapsed() >= INTERVALO_HIST) {
            self.hist_pedido = Some(agora);
            let plat = self.plat.clone();
            let (tx, rx) = mpsc::channel();
            self.rx_hist = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(plat.historicos().and_then(|j| historico::ler_historicos(&j)));
            });
        }

        // 3) eventos, com a aba Eventos aberta (e logo depois de criar/editar/apagar um)
        if self.aba == Aba::Eventos && self.detalhe.is_none() {
            if self.ev_ultimo_check.map_or(true, |t| t.elapsed() >= Duration::from_secs(2)) {
                self.ev_ultimo_check = Some(agora);
                let v = self.plat.eventos_versao();
                if v != self.ev_versao {
                    self.ev_versao = v;
                    self.ev_pedido = None;
                }
            }
            if self.rx_ev.is_none() && self.ev_pedido.map_or(true, |t| t.elapsed() >= INTERVALO_EVENTOS) {
                self.ev_pedido = Some(agora);
                let plat = self.plat.clone();
                let (tx, rx) = mpsc::channel();
                self.rx_ev = Some(rx);
                std::thread::spawn(move || {
                    let _ = tx.send(plat.eventos().and_then(|j| historico::ler_eventos(&j)));
                });
            }
        }

        // 4) "Tudo" do Placar (vem do Leaderboard, que inclui as horas antigas da Steam/PSN)
        if self.aba == Aba::Placar && self.detalhe.is_none() && self.pl_periodo == 3 && self.rx_pl_tudo.is_none() {
            if self.pl_modo == ModoRanking::PorJogo && self.pl_jogo.is_none() {
                self.pl_jogo = self.jogos_placar().first().map(|j| j.0.clone());
            }
            if let Some(chave) = self.pl_chave_tudo() {
                let precisa = match &self.pl_tudo_pedido {
                    None => true,
                    Some((k, t)) => *k != chave || t.elapsed() >= INTERVALO_TUDO,
                };
                if precisa {
                    self.pl_tudo_pedido = Some((chave.clone(), agora));
                    let (m, pl, j) = (
                        self.pl_modo.chave().to_string(),
                        self.pl_plataforma.chave().to_string(),
                        self.pl_jogo.clone().unwrap_or_default(),
                    );
                    let dia = chave_dia();
                    let plat = self.plat.clone();
                    let (tx, rx) = mpsc::channel();
                    self.rx_pl_tudo = Some(rx);
                    std::thread::spawn(move || {
                        let r = plat
                            .buscar_ranking(&m, "tudo", &pl, &j, &dia)
                            .and_then(|t| ranking::ler_itens(&t));
                        let _ = tx.send((chave, r));
                    });
                }
            }
        }

        // 5) dia escolhido no calendário
        if self.pl_esperando_data
            && self
                .pl_ultimo_check_data
                .map_or(true, |t| t.elapsed() >= Duration::from_millis(300))
        {
            self.pl_ultimo_check_data = Some(agora);
            if let Some(d) = self.plat.data_escolhida() {
                self.pl_esperando_data = false;
                if let Some(d) = historico::ler_dia(&d) {
                    self.pl_data = Some(d);
                    self.inicio_dados = self.an.agora;
                }
            }
        }
    }

    fn receber_placar(&mut self) {
        if let Some(rx) = &self.rx_hist_pub {
            match rx.try_recv() {
                Ok(r) => {
                    self.rx_hist_pub = None;
                    if r.is_err() {
                        // tenta de novo na próxima rodada
                        if let Some((j, _)) = &mut self.hist_publicado {
                            j.clear();
                        }
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.rx_hist_pub = None,
            }
        }
        if let Some(rx) = &self.rx_hist {
            match rx.try_recv() {
                Ok(r) => {
                    self.rx_hist = None;
                    match r {
                        Ok(v) => {
                            let primeira = self.hist_quando.is_none();
                            self.hist = v;
                            self.hist_erro = None;
                            self.hist_quando = Some(Instant::now());
                            if primeira {
                                self.inicio_dados = self.an.agora;
                            }
                        }
                        Err(e) => self.hist_erro = Some(e),
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.rx_hist = None,
            }
        }
        if let Some(rx) = &self.rx_ev {
            match rx.try_recv() {
                Ok(r) => {
                    self.rx_ev = None;
                    match r {
                        Ok(v) => {
                            let primeira = self.ev_quando.is_none();
                            self.eventos = v;
                            self.ev_erro = None;
                            self.ev_quando = Some(Instant::now());
                            if primeira {
                                self.inicio_dados = self.an.agora;
                            }
                        }
                        Err(e) => self.ev_erro = Some(e),
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.rx_ev = None,
            }
        }
        if let Some(rx) = &self.rx_pl_tudo {
            match rx.try_recv() {
                Ok((chave, r)) => {
                    self.rx_pl_tudo = None;
                    match r {
                        Ok(v) => {
                            self.pl_tudo.insert(chave, (v, Instant::now()));
                            self.hist_erro = None;
                        }
                        Err(e) => self.hist_erro = Some(e),
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.rx_pl_tudo = None,
            }
        }
    }

    fn pl_chave_tudo(&self) -> Option<String> {
        let jogo = match self.pl_modo {
            ModoRanking::PorJogo => self.pl_jogo.clone()?,
            _ => String::new(),
        };
        Some(format!(
            "{}|{}|{}",
            self.pl_modo.chave(),
            self.pl_plataforma.chave(),
            jogo
        ))
    }

    /// Jogos para escolher no Placar e nos Eventos: os seus e os que aparecem no histórico de alguém.
    pub(super) fn jogos_placar(&self) -> Vec<(String, String, String)> {
        let mut v = self.jogos_para_escolher();
        for j in historico::jogos_conhecidos(&self.hist) {
            if !v.iter().any(|x| x.0 == j.0) {
                v.push(j);
            }
        }
        v
    }

    // =====================================================================
    // Tela
    // =====================================================================

    pub(super) fn tela_placar(&mut self, ui: &mut Ui, topo: f32) {
        let logado = self.conta.estado == EstadoConta::Logado;
        let tp = tipo(self.pl_periodo);
        let hoje = historico::hoje();
        let jogo = self.pl_jogo.clone();
        let carregando = self.hist_quando.is_none() && self.hist_erro.is_none();

        // períodos que aparecem: o escolhido no calendário, ou do mais recente que acabou para trás
        let mut periodos: Vec<(NaiveDate, NaiveDate)> = Vec::new();
        let mut ha_mais = false;
        if let Some(t) = tp {
            if let Some(d) = self.pl_data {
                periodos.push(t.intervalo(d));
            } else if let Some(primeiro) = historico::primeiro_dia(&self.hist) {
                let mut p = t.anterior(t.intervalo(hoje).0);
                while p.1 >= primeiro {
                    if periodos.len() >= self.pl_limite {
                        ha_mais = true;
                        break;
                    }
                    periodos.push(p);
                    p = t.anterior(p.0);
                }
            }
        }
        let modo = self.pl_modo;
        let resultados: Vec<(String, bool, Vec<ItemRanking>)> = match tp {
            Some(t) => periodos
                .iter()
                .map(|(i, f)| {
                    (
                        t.rotulo(*i, *f),
                        *f >= hoje,
                        historico::ranking(
                            &self.hist,
                            &fmt_dia(*i),
                            &fmt_dia(*f),
                            modo,
                            self.pl_plataforma,
                            jogo.as_deref(),
                        ),
                    )
                })
                .collect(),
            None => Vec::new(),
        };
        let tudo: Option<Vec<ItemRanking>> = self
            .pl_chave_tudo()
            .and_then(|c| self.pl_tudo.get(&c))
            .map(|(v, _)| v.clone());

        let eu = self.conta.uid.clone();
        let vitorias = resultados
            .iter()
            .filter(|(_, andamento, v)| !andamento && v.first().is_some_and(|i| i.uid == eu && !eu.is_empty()))
            .count();

        self.hero(ui, topo, |s, ui| {
            s.barra_marca(ui, Some("Placar"));
            if !logado {
                s.rotulo_hero(ui, "Vencedores do conjunto Abyss");
                ui.label(txt("Quem ganhou?", 34.0, Peso::Pesado, s.p.on_hero));
                s.sub_hero(ui, "Entre para ver quem ganhou cada dia, semana e mês.");
                return;
            }
            let mut pl = s.pl_plataforma.indice();
            if desenho::seletor(ui, &mut s.an, &s.p, "pl_plat", &PlataformaRanking::NOMES, &mut pl, true) {
                s.pl_plataforma = PlataformaRanking::de(pl);
                s.pl_limite = LOTE;
                s.inicio_dados = s.an.agora;
            }
            ui.add_space(4.0);
            let mut m = s.pl_modo.indice();
            if desenho::seletor(ui, &mut s.an, &s.p, "pl_modo", &ModoRanking::NOMES, &mut m, true) {
                s.pl_modo = ModoRanking::de(m);
                s.pl_limite = LOTE;
                s.inicio_dados = s.an.agora;
            }
            ui.add_space(4.0);
            let mut p = s.pl_periodo;
            if desenho::seletor(ui, &mut s.an, &s.p, "pl_periodo", &PERIODOS, &mut p, true) {
                s.pl_periodo = p;
                s.pl_limite = LOTE;
                s.inicio_dados = s.an.agora;
            }
            ui.add_space(12.0);

            let (rotulo, lider, sub) = match tp {
                None => {
                    let l = tudo.as_ref().and_then(|v| v.first());
                    (
                        "Na frente agora · Tudo".to_string(),
                        l.map(|i| i.nome.clone()),
                        l.map(|i| format!("{} de jogo no total", fmt_duracao(i.ms))),
                    )
                }
                Some(_) => {
                    let r = resultados.first();
                    let l = r.and_then(|(_, _, v)| v.first());
                    let rot = if s.pl_data.is_some() { "Período escolhido" } else { "Último vencedor" };
                    (
                        rot.to_string(),
                        l.map(|i| i.nome.clone()),
                        r.map(|(t, _, v)| match v.first() {
                            Some(i) => format!("{t} · {}", fmt_duracao(i.ms)),
                            None => t.clone(),
                        }),
                    )
                }
            };
            s.rotulo_hero(ui, &rotulo);
            let pop = s.an.desde(s.inicio_dados, 0.1, 0.5);
            let nome = lider.unwrap_or_else(|| "—".into());
            ui.add(egui::Label::new(txt(nome, 30.0 + 4.0 * pop, Peso::Pesado, s.p.on_hero)).truncate());
            let sub = sub.unwrap_or_else(|| {
                if carregando {
                    "Carregando o placar...".into()
                } else {
                    "Ainda não tem ninguém aqui.".into()
                }
            });
            s.sub_hero(ui, &sub);
            if tp.is_some() {
                let n = resultados.iter().filter(|r| !r.1).count();
                let lugar = resultados
                    .first()
                    .and_then(|(_, _, v)| v.iter().position(|i| i.uid == eu && !eu.is_empty()))
                    .map(|p| format!("#{}", p + 1))
                    .unwrap_or_else(|| "—".into());
                s.pilulas(
                    ui,
                    [
                        ("Você ganhou", format!("{vitorias} de {n}")),
                        ("Seu lugar no último", lugar),
                    ],
                );
            }
        });

        self.conteudo(ui, |s, ui| {
            if !logado {
                s.cartao(ui, 0, |s, ui| s.cartao_login(ui));
                return;
            }
            let mut i = 0;
            if s.pl_modo == ModoRanking::PorJogo {
                let jogos = s.jogos_placar();
                let sel = s.pl_jogo.clone();
                s.cartao(ui, i, |s, ui| {
                    if let Some(c) = s.escolher_jogo_em(ui, "pl", &jogos, sel.as_deref()) {
                        s.pl_jogo = Some(c);
                        s.pl_limite = LOTE;
                        s.inicio_dados = s.an.agora;
                    }
                });
                i += 1;
            }
            if let Some(t) = tp {
                s.cartao(ui, i, |s, ui| {
                    s.titulo_cartao(ui, "Quando?", "");
                    let dica = match s.pl_data {
                        Some(d) => {
                            let (a, b) = t.intervalo(d);
                            format!("Mostrando: {}", t.rotulo(a, b))
                        }
                        None => "Do período mais recente que acabou para o mais antigo.".to_string(),
                    };
                    ui.label(txt(dica, 13.0, Peso::Normal, s.p.muted));
                    let tem_data = s.pl_data.is_some();
                    ui.columns(2, |c| {
                        if botao(&mut c[0], &mut s.an, &s.p, "pl_data", "Escolher data", true).clicked() {
                            let atual = fmt_dia(s.pl_data.unwrap_or(hoje));
                            s.plat.escolher_data(&atual);
                            s.pl_esperando_data = true;
                            s.pl_ultimo_check_data = None;
                        }
                        if tem_data && botao(&mut c[1], &mut s.an, &s.p, "pl_todos", "Ver todos", false).clicked() {
                            s.pl_data = None;
                            s.pl_limite = LOTE;
                            s.inicio_dados = s.an.agora;
                        }
                    });
                });
                i += 1;
            }
            if let Some(e) = s.hist_erro.clone() {
                s.cartao(ui, i, |s, ui| {
                    ui.label(txt("Algo deu errado", 15.0, Peso::Semi, s.p.bad));
                    ui.label(txt(&e, 12.5, Peso::Normal, s.p.muted));
                    if botao(ui, &mut s.an, &s.p, "pl_tentar", "Tentar de novo", false).clicked() {
                        s.hist_pedido = None;
                        s.pl_tudo_pedido = None;
                        s.hist_erro = None;
                    }
                });
                i += 1;
            }

            match tp {
                None => {
                    s.cartao(ui, i, |s, ui| {
                        s.titulo_cartao(ui, "Na frente agora", "Tudo");
                        match &tudo {
                            None => {
                                ui.vertical_centered(|ui| {
                                    ui.add(egui::Spinner::new().size(24.0));
                                });
                            }
                            Some(v) if v.is_empty() => {
                                ui.label(txt("Ninguém por aqui ainda.", 13.5, Peso::Normal, s.p.muted));
                            }
                            Some(v) => {
                                for (k, it) in v.iter().take(3).enumerate() {
                                    s.linha_top(ui, k, it, modo != ModoRanking::PorJogo);
                                }
                            }
                        }
                    });
                }
                Some(_) => {
                    if resultados.is_empty() {
                        s.cartao(ui, i, |s, ui| {
                            if carregando {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(10.0);
                                    ui.add(egui::Spinner::new().size(28.0));
                                    ui.label(txt("Buscando os vencedores...", 13.5, Peso::Normal, s.p.muted));
                                });
                            } else {
                                s.vazio(
                                    ui,
                                    "Nenhum vencedor ainda",
                                    "Quando um período acabar, quem jogou mais aparece aqui.",
                                );
                            }
                        });
                    }
                    let total = resultados.len();
                    for (k, (rotulo, andamento, itens)) in resultados.iter().enumerate() {
                        let resp = s.cartao(ui, i + k, |s, ui| {
                            let dica = if *andamento {
                                "em andamento".to_string()
                            } else if itens.is_empty() {
                                String::new()
                            } else {
                                format!("{} {}", itens.len(), if itens.len() == 1 { "jogador" } else { "jogadores" })
                            };
                            s.titulo_cartao(ui, rotulo, &dica);
                            if itens.is_empty() {
                                ui.label(txt("Ninguém jogou nesse período.", 13.5, Peso::Normal, s.p.muted));
                            }
                            for (pos, it) in itens.iter().take(3).enumerate() {
                                s.linha_top(ui, pos, it, modo != ModoRanking::PorJogo);
                            }
                        });
                        // chegou no último: carrega mais
                        if k + 1 == total && ha_mais && ui.is_rect_visible(resp.rect) {
                            s.pl_limite += LOTE;
                            s.an.ativo = true;
                        }
                    }
                    if ha_mais {
                        ui.vertical_centered(|ui| {
                            ui.label(txt("Carregando mais...", 12.0, Peso::Normal, s.p.muted));
                        });
                    }
                }
            }
        });
    }

    /// Uma colocação (1º, 2º, 3º... com a cor da medalha), com foto, nome, jogo e tempo.
    pub(super) fn linha_top(&mut self, ui: &mut Ui, pos: usize, it: &ItemRanking, mostrar_jogo: bool) {
        let p = self.p;
        let eu = it.uid == self.conta.uid && !self.conta.uid.is_empty();
        ui.horizontal(|ui| {
            ui.set_min_height(44.0);
            let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
            let (bg, fg) = if pos < 3 {
                (cor_medalha(pos), Color32::WHITE)
            } else {
                (p.chip, p.text)
            };
            ui.painter().circle_filled(r.center(), 15.0, bg);
            ui.painter().text(
                r.center(),
                Align2::CENTER_CENTER,
                format!("{}", pos + 1),
                fonte(13.0, Peso::Pesado),
                fg,
            );
            let (rf, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
            let pt = ui.painter().clone();
            if pos < 3 {
                pt.circle_filled(rf.center(), 20.5, alfa(cor_medalha(pos), 0.9));
            }
            self.foto_pessoa(&pt, it, rf.center(), 18.5);
            ui.allocate_ui_with_layout(
                vec2((ui.available_width() - 84.0).max(40.0), 40.0),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let nome = if eu {
                        format!("{} · você", it.nome)
                    } else {
                        it.nome.clone()
                    };
                    ui.add(egui::Label::new(txt(nome, 15.0, Peso::Semi, if eu { p.gold } else { p.text })).truncate());
                    if mostrar_jogo && !it.jogo.is_empty() {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            if !it.pacote.is_empty() {
                                self.icone_jogo(ui, &it.pacote, &it.jogo, 15.0);
                            }
                            ui.add(egui::Label::new(txt(&it.jogo, 12.0, Peso::Normal, p.muted)).truncate());
                        });
                    }
                },
            );
            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                if !it.plats.is_empty() {
                    let (rb, _) = ui.allocate_exact_size(vec2(52.0, 14.0), Sense::hover());
                    let pt = ui.painter().clone();
                    self.selo_plataformas(&pt, &it.plats, rb.right_top(), 0.75, 1.0);
                }
                ui.label(txt(fmt_duracao(it.ms), 14.5, Peso::Pesado, p.text));
            });
        });
    }
}
