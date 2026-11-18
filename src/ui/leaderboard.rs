//! Aba Leaderboard: quem mais jogou (pessoas, pessoa + jogo, ou por jogo).
//! Atualiza sozinha a cada minuto enquanto a aba está aberta e envia o seu
//! tempo de jogo a cada minuto quando você está logado.

use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

use chrono::Local;
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CornerRadius, Layout, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2,
};

use super::desenho::{self, Icone};
use super::telas::{botao, entrada};
use super::tema::{alfa, fonte, misturar_cor, txt, Peso};
use super::*;
use crate::plataforma::{agora_ms, EstadoConta};
use crate::ranking::{self, ItemRanking, ModoRanking, PeriodoRanking, PlataformaRanking};

const INTERVALO: Duration = Duration::from_secs(60);

const OURO: Color32 = Color32::from_rgb(0xe3, 0xad, 0x58);
const PRATA: Color32 = Color32::from_rgb(0xa9, 0xb6, 0xcc);
const BRONZE: Color32 = Color32::from_rgb(0xcd, 0x8b, 0x5a);

pub(crate) struct Resposta {
    pub chave: Option<String>,
    pub itens: Option<Result<Vec<ItemRanking>, String>>,
    pub publicado: Option<Result<(), String>>,
}

pub(crate) fn chave_dia() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn cor_medalha(pos: usize) -> Color32 {
    match pos {
        0 => OURO,
        1 => PRATA,
        _ => BRONZE,
    }
}

impl AbyssHubApp {
    // =====================================================================
    // Dados
    // =====================================================================

    pub(super) fn lb_chave(&self) -> Option<String> {
        let jogo = match self.lb_modo {
            ModoRanking::PorJogo => self.lb_jogo.clone()?,
            _ => String::new(),
        };
        Some(format!(
            "{}|{}|{}|{}",
            self.lb_modo.chave(),
            self.lb_periodo.chave(),
            self.lb_plataforma.chave(),
            jogo
        ))
    }

    /// Jogos para escolher no modo "Por jogo": os seus primeiro, depois os que aparecem no ranking.
    /// Cada item: (chave do jogo, pacote para o ícone, nome).
    pub(super) fn jogos_para_escolher(&self) -> Vec<(String, String, String)> {
        let mut v: Vec<(String, String, String)> = Vec::new();
        for l in self.totais(0, agora_ms()) {
            if !self.eh_jogo(&l.pacote) {
                continue;
            }
            let nome = self.nome(&l.pacote);
            let chave = ranking::chave_jogo(&nome, &l.pacote);
            if !v.iter().any(|(c, _, _)| *c == chave) {
                v.push((chave, l.pacote.clone(), nome));
            }
        }
        for (itens, _) in self.lb_cache.values() {
            for i in itens {
                if !i.chave.is_empty() && !v.iter().any(|(c, _, _)| *c == i.chave) {
                    v.push((i.chave.clone(), i.pacote.clone(), i.jogo.clone()));
                }
            }
        }
        v
    }

    /// O JSON com o seu tempo (só o que conta como jogo, com os repetidos juntados),
    /// separado por período e por plataforma (celular / Steam).
    fn json_publicacao(&self) -> String {
        let conta = |c: &str| c != self.proprio && self.eh_jogo(c);
        let mut sess: Vec<ranking::SessaoPub> = self
            .dados
            .sessoes
            .iter()
            .filter(|s| conta(self.canonico(&s.pacote)))
            .map(|s| ranking::SessaoPub {
                sessao: Sessao {
                    pacote: self.canonico(&s.pacote).to_string(),
                    inicio: s.inicio,
                    fim: s.fim,
                },
                steam: crate::steam::eh_steam(&s.pacote),
            })
            .collect();
        if let Some((p, ini)) = &self.aberta {
            let c = self.canonico(p);
            if conta(c) {
                sess.push(ranking::SessaoPub {
                    sessao: Sessao {
                        pacote: c.to_string(),
                        inicio: *ini,
                        fim: agora_ms(),
                    },
                    steam: false,
                });
            }
        }
        let bases: Vec<(String, i64)> = self
            .dados
            .steam
            .jogos
            .iter()
            .map(|(id, j)| (self.canonico(&crate::steam::pacote(id)).to_string(), j.base_ms))
            .filter(|(c, ms)| *ms > 0 && conta(c.as_str()))
            .collect();
        let iv = ranking::Intervalos {
            agora: agora_ms(),
            hoje: inicio_do_dia(0),
            semana: inicio_do_dia(6),
            mes: inicio_do_dia(29),
        };
        ranking::montar_publicacao(
            &sess,
            &bases,
            |p| self.nome(p),
            |p| self.plataformas_de(p),
            &iv,
            &chave_dia(),
        )
    }

    /// Chamado todo quadro: recebe respostas e dispara a próxima atualização quando der 1 minuto.
    pub(super) fn cuidar_leaderboard(&mut self) {
        if let Some(rx) = &self.lb_rx {
            match rx.try_recv() {
                Ok(r) => {
                    self.lb_rx = None;
                    self.receber_leaderboard(r);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.lb_rx = None,
            }
        }
        if self.conta.estado != EstadoConta::Logado || self.lb_rx.is_some() {
            return;
        }
        if self.lb_modo == ModoRanking::PorJogo && self.lb_jogo.is_none() {
            self.lb_jogo = self.jogos_para_escolher().first().map(|j| j.0.clone());
        }

        let na_aba = self.aba == Aba::Leaderboard && self.detalhe.is_none();
        let chave = self.lb_chave();
        let buscar = na_aba
            && match (&chave, &self.lb_pedido) {
                (None, _) => false,
                (Some(_), None) => true,
                (Some(c), Some((k, t))) => c != k || t.elapsed() >= INTERVALO,
            };

        let mut json_pub = None;
        let vencido = self
            .ultima_publicacao
            .as_ref()
            .map_or(true, |(_, t)| t.elapsed() >= INTERVALO);
        if vencido {
            let json = self.json_publicacao();
            let mudou = self.ultima_publicacao.as_ref().map_or(true, |(j, _)| *j != json);
            if mudou {
                json_pub = Some(json.clone());
            }
            self.ultima_publicacao = Some((json, Instant::now()));
        }

        if !buscar && json_pub.is_none() {
            return;
        }
        let consulta = if buscar {
            let c = chave.unwrap_or_default();
            self.lb_pedido = Some((c.clone(), Instant::now()));
            Some((
                c,
                self.lb_modo.chave().to_string(),
                self.lb_periodo.chave().to_string(),
                self.lb_plataforma.chave().to_string(),
                self.lb_jogo.clone().unwrap_or_default(),
                chave_dia(),
            ))
        } else {
            None
        };

        let plat = self.plat.clone();
        let (tx, rx) = mpsc::channel();
        self.lb_rx = Some(rx);
        std::thread::spawn(move || {
            // primeiro envia o seu tempo, para você já aparecer atualizado na busca
            let publicado = json_pub.map(|j| plat.publicar(&j));
            let itens = consulta
                .as_ref()
                .map(|(_, m, p, pl, j, d)| plat.buscar_ranking(m, p, pl, j, d).and_then(|t| ranking::ler_itens(&t)));
            let _ = tx.send(Resposta {
                chave: consulta.map(|c| c.0),
                itens,
                publicado,
            });
        });
    }

    fn receber_leaderboard(&mut self, r: Resposta) {
        if let Some(Err(e)) = &r.publicado {
            self.ultima_publicacao = None; // tenta de novo no próximo minuto
            self.lb_erro = Some(format!("Não consegui enviar seu tempo: {e}"));
        }
        let (Some(chave), Some(itens)) = (r.chave, r.itens) else {
            return;
        };
        match itens {
            Ok(v) => {
                let mudou = self.lb_cache.get(&chave).map_or(true, |(antigo, _)| *antigo != v);
                if mudou && self.lb_chave().as_deref() == Some(chave.as_str()) {
                    self.inicio_dados = self.an.agora;
                }
                self.lb_cache.insert(chave, (v, Instant::now()));
                if !matches!(r.publicado, Some(Err(_))) {
                    self.lb_erro = None;
                }
            }
            Err(e) => self.lb_erro = Some(e),
        }
    }

    fn atualizar_agora(&mut self) {
        self.lb_pedido = None;
        self.ultima_publicacao = None;
    }

    // =====================================================================
    // Desenho de fotos e ícones
    // =====================================================================

    /// Foto redonda da pessoa (ou a inicial, enquanto carrega / se não tiver foto).
    fn foto_pessoa(&mut self, pt: &Painter, item: &ItemRanking, centro: Pos2, raio: f32) {
        let r = Rect::from_center_size(centro, Vec2::splat(raio * 2.0));
        match self.imagens.foto(&item.foto) {
            Some(tex) => desenho::imagem_recortada(pt, tex, r, raio, Color32::WHITE),
            None => desenho::desenhar_avatar(pt, &self.p, &item.nome, &item.uid, r, self.escuro, true),
        }
    }

    /// Ícone do jogo num retângulo (ou a inicial, se não tiver).
    pub(super) fn icone_jogo_em(&mut self, pt: &Painter, pacote: &str, nome: &str, r: Rect) {
        match self.imagens.icone(pacote) {
            Some(tex) => desenho::imagem_recortada(pt, tex, r, r.width() * 0.25, Color32::WHITE),
            None => desenho::desenhar_avatar(pt, &self.p, nome, pacote, r, self.escuro, false),
        }
    }

    /// Símbolo de onde o jogo existe (celular e/ou Steam), com o canto de cima à direita em `canto`.
    /// Se o jogo está nas duas, aparecem os dois menores, um do lado do outro.
    fn selo_plataformas(&self, pt: &Painter, plats: &[String], canto: Pos2, escala: f32, a: f32) {
        let mut lista = Vec::new();
        if plats.iter().any(|p| p == "mobile") {
            lista.push(Icone::Celular);
        }
        if plats.iter().any(|p| p == "steam") {
            lista.push(Icone::Pc);
        }
        if lista.is_empty() || a <= 0.0 {
            return;
        }
        let tam = escala * if lista.len() == 2 { 16.0 } else { 19.0 };
        let mut x = canto.x;
        for qual in lista.into_iter().rev() {
            let r = Rect::from_min_size(pos2(x - tam, canto.y), Vec2::splat(tam));
            pt.rect_filled(r, CornerRadius::same(5), alfa(self.p.chip, a));
            desenho::icone(pt, qual, r.center(), tam * 0.68, alfa(self.p.muted, a), 0.0);
            x -= tam + 3.0;
        }
    }

    /// Ícone do jogo ocupando um espaço no layout.
    pub(super) fn icone_jogo(&mut self, ui: &mut Ui, pacote: &str, nome: &str, tam: f32) {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(tam), Sense::hover());
        let pt = ui.painter().clone();
        self.icone_jogo_em(&pt, pacote, nome, r);
    }

    // =====================================================================
    // Tela
    // =====================================================================

    pub(super) fn tela_leaderboard(&mut self, ui: &mut Ui, topo: f32) {
        let logado = self.conta.estado == EstadoConta::Logado;
        let chave = self.lb_chave();
        let (itens, quando) = chave
            .as_ref()
            .and_then(|c| self.lb_cache.get(c))
            .map(|(v, t)| (v.clone(), Some(*t)))
            .unwrap_or((Vec::new(), None));
        let minha = itens
            .iter()
            .position(|i| i.uid == self.conta.uid && !self.conta.uid.is_empty());
        let carregando = self.lb_rx.is_some() && quando.is_none();
        let faltam = self
            .lb_pedido
            .as_ref()
            .map(|(_, t)| INTERVALO.saturating_sub(t.elapsed()).as_secs())
            .unwrap_or(0);

        self.hero(ui, topo, |s, ui| {
            s.barra_marca(ui, Some("Leaderboard"));
            if !logado {
                s.rotulo_hero(ui, "Ranking do conjunto Abyss");
                ui.label(txt("Entre e compita", 34.0, Peso::Pesado, s.p.on_hero));
                s.sub_hero(ui, "Veja quem mais jogou e mostre o seu tempo de jogo.");
                return;
            }
            let mut pl = s.lb_plataforma.indice();
            if desenho::seletor(ui, &mut s.an, &s.p, "lb_plat", &PlataformaRanking::NOMES, &mut pl, true) {
                s.lb_plataforma = PlataformaRanking::de(pl);
                s.inicio_dados = s.an.agora;
            }
            ui.add_space(4.0);
            let mut m = s.lb_modo.indice();
            if desenho::seletor(ui, &mut s.an, &s.p, "lb_modo", &ModoRanking::NOMES, &mut m, true) {
                s.lb_modo = ModoRanking::de(m);
                s.inicio_dados = s.an.agora;
            }
            ui.add_space(4.0);
            let mut p = s.lb_periodo.indice();
            if desenho::seletor(ui, &mut s.an, &s.p, "lb_periodo", &PeriodoRanking::NOMES, &mut p, true) {
                s.lb_periodo = PeriodoRanking::de(p);
                s.inicio_dados = s.an.agora;
            }
            ui.add_space(12.0);
            let filtro = match s.lb_plataforma {
                PlataformaRanking::Todas => s.lb_periodo.nome().to_string(),
                pl => format!("{} · {}", s.lb_periodo.nome(), pl.nome()),
            };
            s.rotulo_hero(ui, &format!("Sua posição · {filtro}"));
            let pos = match minha {
                Some(i) => format!("#{}", i + 1),
                None => "—".into(),
            };
            let pop = s.an.desde(s.inicio_dados, 0.1, 0.5);
            ui.label(txt(pos, 34.0 + 6.0 * pop, Peso::Pesado, s.p.on_hero));
            let sub = match minha {
                Some(i) => {
                    let it = &itens[i];
                    let jogo = if s.lb_modo == ModoRanking::Pessoas {
                        String::new()
                    } else {
                        format!(" em {}", it.jogo)
                    };
                    format!("{}{jogo} · {} no ranking", fmt_duracao(it.ms), itens.len())
                }
                None if carregando => "Carregando o ranking...".into(),
                None => "Jogue um pouco para aparecer aqui.".into(),
            };
            s.sub_hero(ui, &sub);
            let lider = itens.first().map(|i| i.nome.clone()).unwrap_or_else(|| "—".into());
            let prox = if s.lb_rx.is_some() {
                "agora".to_string()
            } else {
                format!("{faltam} s")
            };
            s.pilulas(ui, [("Líder", lider), ("Próxima atualização", prox)]);
        });

        self.conteudo(ui, |s, ui| {
            if !logado {
                s.cartao(ui, 0, |s, ui| s.cartao_login(ui));
                return;
            }
            let mut i_cartao = 0;
            if s.lb_modo == ModoRanking::PorJogo {
                s.cartao(ui, i_cartao, |s, ui| s.escolher_jogo(ui));
                i_cartao += 1;
            }
            if let Some(e) = s.lb_erro.clone() {
                s.cartao(ui, i_cartao, |s, ui| {
                    ui.label(txt("Algo deu errado", 15.0, Peso::Semi, s.p.bad));
                    ui.label(txt(&e, 12.5, Peso::Normal, s.p.muted));
                    if let Some(link) = e.split_whitespace().find(|w| w.starts_with("https://")) {
                        let link = link.trim_end_matches(['.', ',', ')']).to_string();
                        if botao(ui, &mut s.an, &s.p, "indice", "Criar índice no Firebase", true).clicked() {
                            s.plat.abrir_link(&link);
                        }
                    }
                });
                i_cartao += 1;
            }

            if itens.is_empty() {
                s.cartao(ui, i_cartao, |s, ui| {
                    if carregando {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.add(egui::Spinner::new().size(28.0));
                            ui.label(txt("Buscando quem mais jogou...", 13.5, Peso::Normal, s.p.muted));
                        });
                    } else {
                        s.vazio(
                            ui,
                            "Ninguém por aqui ainda",
                            "Quando alguém jogar neste período, aparece aqui.",
                        );
                    }
                });
                return;
            }

            s.cartao(ui, i_cartao, |s, ui| s.podio(ui, &itens));
            if itens.len() > 3 {
                s.cartao(ui, i_cartao + 1, |s, ui| {
                    s.titulo_cartao(ui, "Classificação", &format!("{} jogadores", itens.len()));
                    for (i, it) in itens.iter().enumerate().skip(3) {
                        s.linha_ranking(ui, i, it);
                    }
                });
            }
            let quando_txt = quando.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            ui.vertical_centered(|ui| {
                let r = ui.add(
                    egui::Label::new(txt(
                        format!("Atualiza sozinho a cada minuto · atualizado há {quando_txt} s · toque para atualizar"),
                        12.0,
                        Peso::Normal,
                        s.p.muted,
                    ))
                    .sense(Sense::click()),
                );
                if r.clicked() {
                    s.atualizar_agora();
                }
            });
        });
        // o contador "próxima atualização" anda sozinho
        self.an.ativo |= logado && self.aba == Aba::Leaderboard;
    }

    fn cartao_login(&mut self, ui: &mut Ui) {
        let p = self.p;
        if self.conta.estado == EstadoConta::Indisponivel {
            ui.label(txt("Leaderboard indisponível", 17.0, Peso::Negrito, p.text));
            ui.label(txt(
                "Esta versão do app foi gerada sem o login. Baixe a versão oficial na página do AbyssHub no GitHub.",
                13.5,
                Peso::Normal,
                p.muted,
            ));
            if !self.conta.msg.is_empty() {
                ui.label(txt(&self.conta.msg, 12.5, Peso::Normal, p.muted));
            }
            return;
        }
        ui.label(txt("Entre para competir", 17.0, Peso::Negrito, p.text));
        ui.label(txt(
            "Só aparece no Leaderboard quem entra. Os outros jogadores veem seu nome, sua foto e o tempo dos seus jogos.",
            13.5,
            Peso::Normal,
            p.muted,
        ));
        ui.add_space(4.0);
        self.botao_google(ui);
        self.botao_email(ui);
        if self.conta.estado == EstadoConta::Erro && !self.conta.msg.is_empty() {
            ui.label(txt(&self.conta.msg, 12.5, Peso::Normal, p.bad));
        }
    }

    /// Botão "Entrar com Google" (branco no claro, azul-escuro no escuro).
    pub(super) fn botao_google(&mut self, ui: &mut Ui) {
        let p = self.p;
        let entrando = self.conta.estado == EstadoConta::Entrando;
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::click());
        let aperto = self.an.ir(
            "btn_google",
            if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 },
            0.15,
        );
        let rr = Rect::from_center_size(r.center(), r.size() * (1.0 - 0.02 * aperto));
        let pt = ui.painter();
        let fundo = misturar_cor(Color32::WHITE, p.chip, self.escuro);
        pt.rect_filled(rr, CornerRadius::same(14), p.line);
        pt.rect_filled(rr.shrink(1.0), CornerRadius::same(13), fundo);
        let texto = if entrando { "Entrando..." } else { "Entrar com Google" };
        let g = pt.layout_no_wrap(texto.to_string(), fonte(15.0, Peso::Negrito), p.text);
        let largura = 22.0 + 12.0 + g.size().x;
        let x0 = rr.center().x - largura / 2.0;
        if entrando {
            let giro = (self.an.agora * 5.0) as f32;
            desenho::icone(
                pt,
                Icone::Atualizar,
                pos2(x0 + 11.0, rr.center().y),
                22.0,
                p.muted,
                giro,
            );
            self.an.ativo = true;
        } else {
            desenho::logo_google(pt, pos2(x0 + 11.0, rr.center().y), 10.0);
        }
        pt.galley(pos2(x0 + 34.0, rr.center().y - g.size().y / 2.0), g, p.text);
        if resp.clicked() && !entrando {
            self.plat.entrar();
            self.conta.estado = EstadoConta::Entrando;
            self.ultimo_check_conta = None;
        }
    }

    /// Botão "Entrar com e-mail" (abre a janelinha do Android para digitar e-mail e senha).
    pub(super) fn botao_email(&mut self, ui: &mut Ui) {
        let p = self.p;
        if botao(ui, &mut self.an, &p, "btn_email", "Entrar com e-mail", false).clicked()
            && self.conta.estado != EstadoConta::Entrando
        {
            self.plat.entrar_email();
            self.conta.estado = EstadoConta::Entrando;
            self.ultimo_check_conta = None;
        }
    }

    fn escolher_jogo(&mut self, ui: &mut Ui) {
        self.titulo_cartao(ui, "Escolha o jogo", "deslize para o lado");
        let jogos = self.jogos_para_escolher();
        if jogos.is_empty() {
            ui.label(txt(
                "Nenhum jogo ainda. Jogue algo primeiro!",
                13.5,
                Peso::Normal,
                self.p.muted,
            ));
            return;
        }
        let p = self.p;
        egui::ScrollArea::horizontal().id_salt("lb_jogos").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (chave, pacote, nome) in &jogos {
                    let sel = self.lb_jogo.as_deref() == Some(chave.as_str());
                    let t = self.an.ir(&format!("lbj:{chave}"), if sel { 1.0 } else { 0.0 }, 0.25);
                    let nome_curto: String = nome.chars().take(18).collect();
                    let g = ui.painter().layout_no_wrap(nome_curto, fonte(13.5, Peso::Semi), p.text);
                    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 50.0, 40.0), Sense::click());
                    let pt = ui.painter().clone();
                    pt.rect_filled(r, CornerRadius::same(20), misturar_cor(p.chip, p.chip_on, t));
                    self.icone_jogo_em(
                        &pt,
                        pacote,
                        nome,
                        Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), vec2(26.0, 26.0)),
                    );
                    pt.galley(
                        pos2(r.left() + 40.0, r.center().y - g.size().y / 2.0),
                        g,
                        misturar_cor(p.text, p.chip_on_ink, t),
                    );
                    if resp.clicked() && !sel {
                        self.lb_jogo = Some(chave.clone());
                        self.inicio_dados = self.an.agora;
                    }
                }
            });
        });
    }

    /// Os 3 primeiros em cima de "degraus" (o do meio é o 1º).
    fn podio(&mut self, ui: &mut Ui, itens: &[ItemRanking]) {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 236.0), Sense::hover());
        let pt = ui.painter().clone();
        let w = r.width() / 3.0;
        // coluna (esquerda, meio, direita) -> posição no ranking
        let colunas = [(1usize, 0usize), (0, 1), (2, 2)];
        let alturas = [74.0, 52.0, 38.0];
        for (pos, col) in colunas {
            let Some(it) = itens.get(pos) else { continue };
            let cresce = self.an.desde(self.inicio_dados, 0.12 + col as f32 * 0.07, 0.75);
            let aparece = self.an.desde(self.inicio_dados, 0.25 + pos as f32 * 0.08, 0.5);
            let x0 = r.left() + w * col as f32;
            let cx = x0 + w / 2.0;
            let cor = cor_medalha(pos);
            let eu = it.uid == self.conta.uid;

            // degrau
            let h = alturas[pos] * cresce;
            let degrau = Rect::from_min_max(pos2(x0 + 8.0, r.bottom() - h), pos2(x0 + w - 8.0, r.bottom()));
            pt.rect_filled(
                degrau,
                CornerRadius {
                    nw: 12,
                    ne: 12,
                    sw: 4,
                    se: 4,
                },
                alfa(cor, 0.22 + 0.1 * self.escuro),
            );
            pt.text(
                degrau.center(),
                Align2::CENTER_CENTER,
                (pos + 1).to_string(),
                fonte(22.0, Peso::Pesado),
                alfa(cor, cresce),
            );

            // textos acima do degrau
            let base = r.bottom() - alturas[pos] - 8.0;
            let clip = pt.with_clip_rect(Rect::from_min_max(
                pos2(x0 + 4.0, r.top()),
                pos2(x0 + w - 4.0, r.bottom()),
            ));
            let a = |c: Color32| alfa(c, aparece);
            clip.text(
                pos2(cx, base),
                Align2::CENTER_BOTTOM,
                fmt_duracao(it.ms),
                fonte(15.5, Peso::Pesado),
                a(self.p.text),
            );
            // jogo: ícone pequeno + nome
            let gj = clip.layout_no_wrap(it.jogo.clone(), fonte(11.5, Peso::Normal), a(self.p.muted));
            let largura_jogo = (gj.size().x + 20.0).min(w - 12.0);
            let jx = cx - largura_jogo / 2.0;
            let jy = base - 30.0;
            if !it.pacote.is_empty() {
                let rr = Rect::from_min_size(pos2(jx, jy - 8.0), vec2(16.0, 16.0));
                self.icone_jogo_em(&clip, &it.pacote, &it.jogo, rr);
            }
            clip.galley(pos2(jx + 20.0, jy - gj.size().y / 2.0), gj, a(self.p.muted));
            let nome = if eu {
                "Você".to_string()
            } else {
                it.nome.split(' ').next().unwrap_or("").to_string()
            };
            clip.text(
                pos2(cx, base - 42.0),
                Align2::CENTER_BOTTOM,
                nome,
                fonte(14.5, Peso::Negrito),
                a(if eu { self.p.gold } else { self.p.text }),
            );

            // onde o jogo existe (celular / Steam), no canto de cima à direita da coluna
            self.selo_plataformas(&pt, &it.plats, pos2(x0 + w - 6.0, r.top() + 2.0), 1.0, aparece);

            // foto com anel da medalha
            let raio = if pos == 0 { 32.0 } else { 26.0 };
            let c = pos2(cx, base - 64.0 - raio + 8.0 * (1.0 - aparece));
            pt.circle_filled(c, raio + 3.5, alfa(cor, aparece));
            self.foto_pessoa(&pt, it, c, raio);
            // medalha
            let m = c + vec2(raio * 0.72, raio * 0.72);
            pt.circle_filled(m, 10.0, alfa(cor, aparece));
            pt.text(
                m,
                Align2::CENTER_CENTER,
                (pos + 1).to_string(),
                fonte(11.0, Peso::Pesado),
                alfa(Color32::WHITE, aparece),
            );
            if pos == 0 {
                desenho::icone(
                    &pt,
                    Icone::Trofeu,
                    c - vec2(0.0, raio + 16.0),
                    22.0,
                    alfa(OURO, aparece),
                    0.0,
                );
            }
        }
    }

    /// Linha da classificação (do 4º em diante).
    fn linha_ranking(&mut self, ui: &mut Ui, i: usize, it: &ItemRanking) {
        let pin = self
            .an
            .desde(self.inicio_dados, 0.3 + (i.min(18) - 3) as f32 * 0.035, 0.4);
        let p = self.p;
        let eu = it.uid == self.conta.uid;
        let modo = self.lb_modo;
        entrada(ui, pin, 0.0, 10.0, |ui| {
            let topo = ui.cursor().top();
            if i > 3 {
                ui.painter()
                    .hline(ui.max_rect().x_range(), topo, Stroke::new(1.0_f32, p.line));
            }
            let fundo = ui.painter().add(Shape::Noop);
            ui.add_space(9.0);
            let linha = ui
                .horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(40.0, 30.0), Sense::hover());
                    let (bg, fg) = if eu { (p.soon_bg, p.soon_ink) } else { (p.chip, p.text) };
                    ui.painter().rect_filled(r, CornerRadius::same(10), bg);
                    ui.painter().text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        format!("{}", i + 1),
                        fonte(13.0, Peso::Negrito),
                        fg,
                    );

                    let (rf, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                    let pt = ui.painter().clone();
                    self.foto_pessoa(&pt, it, rf.center(), 20.0);

                    ui.allocate_ui_with_layout(
                        vec2((ui.available_width() - 84.0).max(40.0), 40.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 3.0;
                            let nome = if eu {
                                format!("{} · você", it.nome)
                            } else {
                                it.nome.clone()
                            };
                            ui.add(
                                egui::Label::new(txt(nome, 15.5, Peso::Semi, if eu { p.gold } else { p.text }))
                                    .truncate(),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                if modo == ModoRanking::PorJogo {
                                    ui.label(txt(
                                        format!("atualizado {}", fmt_relativo(it.atualizado_ms)),
                                        12.0,
                                        Peso::Normal,
                                        p.muted,
                                    ));
                                } else {
                                    if !it.pacote.is_empty() {
                                        self.icone_jogo(ui, &it.pacote, &it.jogo, 16.0);
                                    }
                                    let pre = if modo == ModoRanking::Pessoas {
                                        "mais jogou: "
                                    } else {
                                        ""
                                    };
                                    ui.add(
                                        egui::Label::new(txt(format!("{pre}{}", it.jogo), 12.0, Peso::Normal, p.muted))
                                            .truncate(),
                                    );
                                }
                            });
                        },
                    );
                    // tempo, com o símbolo da plataforma (celular / Steam) em cima, à direita
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        if !it.plats.is_empty() {
                            let (rb, _) = ui.allocate_exact_size(vec2(40.0, 15.0), Sense::hover());
                            let pt = ui.painter().clone();
                            self.selo_plataformas(&pt, &it.plats, rb.right_top(), 0.8, 1.0);
                        } else {
                            ui.add_space(8.0);
                        }
                        ui.label(txt(fmt_duracao(it.ms), 15.5, Peso::Pesado, p.text));
                    });
                })
                .response
                .rect;
            ui.add_space(7.0);
            if eu {
                let area = Rect::from_min_max(
                    pos2(linha.left() - 6.0, topo + 2.0),
                    pos2(linha.right() + 6.0, ui.cursor().top() - 2.0),
                );
                ui.painter().set(
                    fundo,
                    Shape::rect_filled(area, CornerRadius::same(12), alfa(p.soon_bg, 0.55)),
                );
            }
        });
    }
}
