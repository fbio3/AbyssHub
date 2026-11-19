//! Interface do AbyssHub (egui), no mesmo estilo do AbyssSub.

mod anim;
mod atualizacao;
mod desenho;
mod eventos;
mod imagens;
mod leaderboard;
mod placar;
mod rolagem;
mod telas;
mod tema;

use std::collections::HashMap;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{Datelike, Days, Local, TimeZone};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};

use crate::dados::Dados;
use crate::historico::{Evento, HistoricoPessoa};
use crate::plataforma::{agora_ms, Conta, EstadoConta, EstadoFonte, InfoApp, Plataforma};
use crate::steam::Fonte;
use crate::ranking::{ItemRanking, ModoRanking, PeriodoRanking, PlataformaRanking};
use crate::sessoes::{self, Sessao, MIN_SESSAO_MS};

use anim::Animador;
use tema::Paleta;

pub(crate) const DIA_MS: i64 = 86_400_000;
pub(crate) const VERSAO: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Periodo {
    Hoje,
    Semana,
    Mes,
    Tudo,
}

impl Periodo {
    pub const NOMES: [&'static str; 4] = ["Hoje", "7 dias", "30 dias", "Tudo"];

    pub fn indice(self) -> usize {
        self as usize
    }

    pub fn de(i: usize) -> Periodo {
        match i {
            0 => Periodo::Hoje,
            1 => Periodo::Semana,
            2 => Periodo::Mes,
            _ => Periodo::Tudo,
        }
    }

    pub fn nome(self) -> &'static str {
        Self::NOMES[self.indice()]
    }

    pub fn inicio(self) -> i64 {
        match self {
            Periodo::Hoje => inicio_do_dia(0),
            Periodo::Semana => inicio_do_dia(6),
            Periodo::Mes => inicio_do_dia(29),
            Periodo::Tudo => 0,
        }
    }

    pub fn dias(self) -> i64 {
        match self {
            Periodo::Hoje => 1,
            Periodo::Semana => 7,
            Periodo::Mes => 30,
            Periodo::Tudo => 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Aba {
    Inicio,
    Jogos,
    Leaderboard,
    Placar,
    Eventos,
    Historico,
}

impl Aba {
    fn indice(self) -> i32 {
        match self {
            Aba::Inicio => 0,
            Aba::Jogos => 1,
            Aba::Leaderboard => 2,
            Aba::Placar => 3,
            Aba::Eventos => 4,
            Aba::Historico => 5,
        }
    }
}

enum Msg {
    Pronto {
        apps: Option<Vec<InfoApp>>,
        fechadas: Vec<Sessao>,
        aberta: Option<(String, i64)>,
        ate: i64,
    },
    Erro(String),
}

pub(crate) struct Linha {
    pub pacote: String,
    pub tempo: i64,
    pub ultima: i64,
    pub sessoes: usize,
}

/// Estado da Steam ou da PSN na tela.
#[derive(Default)]
pub(crate) struct Externo {
    pub(crate) estado: EstadoFonte,
    pub(crate) ultimo_check: Option<Instant>,
    pub(crate) rx: Option<mpsc::Receiver<Result<Option<String>, String>>>,
    pub(crate) ultima_sync: Option<Instant>,
    pub(crate) manual: bool,
    pub(crate) erro: Option<String>,
    /// "Desconectar" pede um segundo toque para confirmar (guarda quando foi o primeiro)
    pub(crate) confirmar_saida: Option<f64>,
}

pub struct AbyssHubApp {
    plat: Arc<dyn Plataforma>,
    proprio: String,
    dados: Dados,
    apps: HashMap<String, InfoApp>,

    periodo: Periodo,
    aba: Aba,
    detalhe: Option<String>,
    ajustes_aberto: bool,
    /// nas configurações, mostrar a lista inteira de apps (e não só os 10 primeiros)
    ajustes_todos_apps: bool,
    dia_sel: Option<usize>,

    permissao: bool,
    ultimo_check: Instant,
    rx: Option<mpsc::Receiver<Msg>>,
    ultima_sync: Option<Instant>,
    precisa_apps: bool,
    sync_manual: bool,
    aberta: Option<(String, i64)>,
    toast: Option<(String, f64, bool)>,

    // aparência
    an: Animador,
    p: Paleta,
    escuro: f32,
    estilo_aplicado: f32,
    escuro_sistema: Option<bool>,
    ultimo_check_tema: Option<Instant>,
    margens: Option<(f32, f32)>,
    ultimo_check_margens: Option<Instant>,
    logo: Option<TextureHandle>,

    // animação de entrada das telas
    boot: Option<f64>,
    inicio_pagina: f64,
    inicio_dados: f64,
    direcao: f32,

    // conta e Leaderboard
    imagens: imagens::Imagens,
    conta: Conta,
    ultimo_check_conta: Option<Instant>,
    lb_modo: ModoRanking,
    lb_periodo: PeriodoRanking,
    lb_plataforma: PlataformaRanking,
    lb_jogo: Option<String>,

    // Placar (filtros)
    pl_modo: ModoRanking,
    /// 0 = dia, 1 = semana, 2 = mês, 3 = tudo
    pl_periodo: usize,
    pl_plataforma: PlataformaRanking,
    pl_jogo: Option<String>,
    /// dia escolhido no calendário (None = todos os períodos, do mais recente para trás)
    pl_data: Option<chrono::NaiveDate>,
    /// quantos períodos já aparecem (vai aumentando conforme a pessoa desce)
    pl_limite: usize,
    pl_esperando_data: bool,
    pl_ultimo_check_data: Option<Instant>,
    /// "Tudo" do Placar vem do Leaderboard: chave -> (lista, quando)
    pl_tudo: HashMap<String, (Vec<ItemRanking>, Instant)>,
    pl_tudo_pedido: Option<(String, Instant)>,
    rx_pl_tudo: Option<mpsc::Receiver<(String, Result<Vec<ItemRanking>, String>)>>,

    // histórico de todo mundo (Placar e Eventos)
    hist: Vec<HistoricoPessoa>,
    hist_quando: Option<Instant>,
    hist_pedido: Option<Instant>,
    hist_erro: Option<String>,
    rx_hist: Option<mpsc::Receiver<Result<Vec<HistoricoPessoa>, String>>>,
    /// o seu histórico já enviado (json, quando)
    hist_publicado: Option<(String, Instant)>,
    rx_hist_pub: Option<mpsc::Receiver<Result<(), String>>>,

    // eventos
    eventos: Vec<Evento>,
    ev_quando: Option<Instant>,
    ev_pedido: Option<Instant>,
    ev_erro: Option<String>,
    ev_versao: i64,
    ev_ultimo_check: Option<Instant>,
    rx_ev: Option<mpsc::Receiver<Result<Vec<Evento>, String>>>,
    /// resultados por consulta (modo/período/jogo) e quando chegaram
    lb_cache: HashMap<String, (Vec<ItemRanking>, Instant)>,
    lb_rx: Option<mpsc::Receiver<leaderboard::Resposta>>,
    lb_pedido: Option<(String, Instant)>,
    lb_erro: Option<String>,
    ultima_publicacao: Option<(String, Instant)>,
    rx_apagar: Option<mpsc::Receiver<Result<(), String>>>,

    // atualização automática
    rx_upd: Option<mpsc::Receiver<Result<Option<crate::atualizacao::Versao>, String>>>,
    upd_manual: bool,
    upd_tentou: bool,
    ultima_tentativa_upd: Option<Instant>,
    /// quando o último quadro foi desenhado (para saber se o app voltou do segundo plano)
    ultimo_quadro: Option<Instant>,
    upd_aberto: bool,
    upd_baixa: crate::atualizacao::Baixa,
    ultimo_poll_upd: Option<Instant>,

    /// Steam e PSN (índice = Fonte)
    externos: [Externo; 2],

    /// posição e velocidade da rolagem de cada tela
    rolagens: HashMap<String, rolagem::Rolagem>,

    /// Mesmo jogo instalado de lojas diferentes (ex.: Roblox da Play Store e da Galaxy Store):
    /// pacote -> pacote "principal", e principal -> todos os pacotes dele.
    canon: HashMap<String, String>,
    apelidos: HashMap<String, Vec<String>>,
}

impl AbyssHubApp {
    pub fn novo(cc: &eframe::CreationContext<'_>, plat: Arc<dyn Plataforma>) -> Self {
        tema::instalar_fontes(&cc.egui_ctx);
        let dados = Dados::carregar(&plat.pasta_dados());
        let permissao = plat.tem_permissao();

        let bytes = include_bytes!("../assets/logo_128.rgba");
        let logo = cc.egui_ctx.load_texture(
            "logo",
            ColorImage::from_rgba_unmultiplied([128, 128], bytes),
            TextureOptions::LINEAR,
        );

        let plat_img = plat.clone();
        let pasta_img = plat.pasta_dados().join("imagens");
        // o último ranking visto fica salvo: aparece na hora ao abrir (ou depois de atualizar) o app
        let lb_cache = leaderboard::carregar_cache(&plat.pasta_dados());
        let mut app = AbyssHubApp {
            proprio: plat.pacote_proprio(),
            plat,
            dados,
            apps: HashMap::new(),
            periodo: Periodo::Semana,
            aba: Aba::Inicio,
            detalhe: None,
            ajustes_aberto: false,
            ajustes_todos_apps: false,
            dia_sel: None,
            permissao,
            ultimo_check: Instant::now(),
            rx: None,
            ultima_sync: None,
            precisa_apps: true,
            sync_manual: false,
            aberta: None,
            toast: None,
            an: Animador::default(),
            p: tema::ESCURO,
            escuro: 1.0,
            estilo_aplicado: -1.0,
            escuro_sistema: None,
            ultimo_check_tema: None,
            margens: None,
            ultimo_check_margens: None,
            logo: Some(logo),
            boot: None,
            inicio_pagina: 0.0,
            inicio_dados: 0.0,
            direcao: 0.0,
            imagens: imagens::Imagens::nova(plat_img, pasta_img),
            conta: Conta::default(),
            ultimo_check_conta: None,
            lb_modo: ModoRanking::Pessoas,
            lb_periodo: PeriodoRanking::Semana,
            lb_plataforma: PlataformaRanking::Todas,
            lb_jogo: None,
            pl_modo: ModoRanking::Pessoas,
            pl_periodo: 1,
            pl_plataforma: PlataformaRanking::Todas,
            pl_jogo: None,
            pl_data: None,
            pl_limite: 6,
            pl_esperando_data: false,
            pl_ultimo_check_data: None,
            pl_tudo: HashMap::new(),
            pl_tudo_pedido: None,
            rx_pl_tudo: None,
            hist: Vec::new(),
            hist_quando: None,
            hist_pedido: None,
            hist_erro: None,
            rx_hist: None,
            hist_publicado: None,
            rx_hist_pub: None,
            eventos: Vec::new(),
            ev_quando: None,
            ev_pedido: None,
            ev_erro: None,
            ev_versao: 0,
            ev_ultimo_check: None,
            rx_ev: None,
            lb_cache,
            lb_rx: None,
            lb_pedido: None,
            lb_erro: None,
            ultima_publicacao: None,
            rx_apagar: None,
            rx_upd: None,
            upd_manual: false,
            upd_tentou: false,
            ultima_tentativa_upd: None,
            ultimo_quadro: None,
            upd_aberto: false,
            upd_baixa: Default::default(),
            ultimo_poll_upd: None,
            externos: [Externo::default(), Externo::default()],
            rolagens: HashMap::new(),
            canon: HashMap::new(),
            apelidos: HashMap::new(),
        };
        app.recalcular_jogos_iguais();
        app
    }

    // ---------- Navegação ----------

    fn trocar_pagina(&mut self, direcao: f32) {
        self.inicio_pagina = self.an.agora;
        self.inicio_dados = self.an.agora;
        self.direcao = direcao;
        self.dia_sel = None;
        // o número grande do topo volta a "contar" quando a tela muda
        self.an.esquecer_prefixo("total:");
    }

    pub(crate) fn ir_para_aba(&mut self, aba: Aba) {
        if self.detalhe.is_some() {
            self.detalhe = None;
            self.aba = aba;
            self.trocar_pagina(-1.0);
        } else if self.aba != aba {
            let dir = (aba.indice() - self.aba.indice()).signum() as f32;
            self.aba = aba;
            self.trocar_pagina(dir);
        }
    }

    pub(crate) fn abrir_detalhe(&mut self, pacote: &str) {
        self.detalhe = Some(pacote.to_string());
        self.trocar_pagina(1.0);
    }

    pub(crate) fn fechar_detalhe(&mut self) {
        self.detalhe = None;
        self.trocar_pagina(-1.0);
    }

    pub(crate) fn mudar_periodo(&mut self, p: Periodo) {
        if self.periodo != p {
            self.periodo = p;
            self.inicio_dados = self.an.agora;
            self.dia_sel = None;
        }
    }

    pub(crate) fn mostrar_toast(&mut self, texto: impl Into<String>, erro: bool) {
        self.toast = Some((texto.into(), self.an.agora, erro));
    }

    // ---------- Sincronização (numa thread separada) ----------

    pub(crate) fn sincronizar(&mut self) {
        if self.rx.is_some() {
            return;
        }
        self.ultima_sync = Some(Instant::now());
        let plat = self.plat.clone();
        let agora = agora_ms();
        let desde = if self.dados.sincronizado_ate > 0 {
            self.dados.sincronizado_ate - 1_000
        } else {
            agora - 30 * DIA_MS
        };
        let com_apps = self.precisa_apps || self.apps.is_empty();
        self.precisa_apps = false;

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let r = (|| -> Result<Msg, String> {
                let apps = if com_apps { Some(plat.listar_apps()?) } else { None };
                let evs = plat.eventos(desde, agora)?;
                let res = sessoes::montar_sessoes(&evs);
                Ok(Msg::Pronto {
                    apps,
                    fechadas: res.fechadas,
                    aberta: res.aberta,
                    ate: agora,
                })
            })();
            let _ = tx.send(r.unwrap_or_else(Msg::Erro));
        });
    }

    fn receber(&mut self) {
        let Some(rx) = &self.rx else { return };
        match rx.try_recv() {
            Ok(Msg::Pronto {
                apps,
                fechadas,
                aberta,
                ate,
            }) => {
                self.rx = None;
                if let Some(apps) = apps {
                    for a in &apps {
                        if a.jogo {
                            self.dados.auto_jogo.insert(a.pacote.clone());
                        }
                    }
                    self.apps = apps.into_iter().map(|a| (a.pacote.clone(), a)).collect();
                    // guarda o nome de todos os jogos, para juntar os repetidos mesmo desinstalados
                    for a in self.apps.values() {
                        if a.jogo || self.dados.nomes.contains_key(&a.pacote) {
                            self.dados.nomes.insert(a.pacote.clone(), a.nome.clone());
                        }
                    }
                }
                let mut novas = 0;
                for s in fechadas {
                    if s.pacote == self.proprio || s.duracao() < MIN_SESSAO_MS {
                        continue;
                    }
                    if let Some(a) = self.apps.get(&s.pacote) {
                        self.dados.nomes.insert(s.pacote.clone(), a.nome.clone());
                    }
                    sessoes::mesclar(&mut self.dados.sessoes, s);
                    novas += 1;
                }
                self.recalcular_jogos_iguais();
                self.dados.sincronizado_ate = aberta.as_ref().map(|a| a.1).unwrap_or(ate);
                self.aberta = aberta.filter(|a| a.0 != self.proprio);
                self.salvar();
                if self.sync_manual {
                    self.sync_manual = false;
                    let msg = if novas == 0 {
                        "Tudo em dia".to_string()
                    } else {
                        format!("Atualizado · {novas} {}", if novas == 1 { "sessão" } else { "sessões" })
                    };
                    self.mostrar_toast(msg, false);
                    self.inicio_dados = self.an.agora;
                }
            }
            Ok(Msg::Erro(e)) => {
                self.rx = None;
                self.sync_manual = false;
                self.mostrar_toast(e, true);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                self.sync_manual = false;
                self.mostrar_toast("A atualização parou no meio", true);
            }
        }
    }

    /// Busca os jogos da Steam ou da PSN numa thread separada.
    pub(crate) fn sincronizar_fonte(&mut self, fonte: Fonte, manual: bool) {
        let ex = &mut self.externos[fonte as usize];
        if ex.rx.is_some() || !ex.estado.conectado() {
            return;
        }
        ex.ultima_sync = Some(Instant::now());
        ex.manual = manual;
        let plat = self.plat.clone();
        let (tx, rx) = mpsc::channel();
        ex.rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(plat.fonte_jogos(fonte));
        });
    }

    /// Atualiza todas as plataformas conectadas (botão de atualizar).
    pub(crate) fn sincronizar_fontes(&mut self) {
        for f in Fonte::TODAS {
            self.sincronizar_fonte(f, false);
        }
    }

    fn receber_fonte(&mut self, fonte: Fonte) {
        let i = fonte as usize;
        let Some(rx) = &self.externos[i].rx else { return };
        let r = match rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(format!("A leitura da {} parou no meio", fonte.nome())),
        };
        self.externos[i].rx = None;
        let manual = std::mem::take(&mut self.externos[i].manual);
        match r.and_then(|j| j.map(|j| crate::steam::ler(&j)).transpose()) {
            Ok(Some((id, jogos))) => {
                let primeira = !fonte.dados(&self.dados).iniciado;
                let novas = crate::steam::aplicar(&mut self.dados, fonte, &id, &jogos, agora_ms());
                self.externos[i].erro = None;
                self.recalcular_jogos_iguais();
                self.salvar();
                self.ultima_publicacao = None; // manda o tempo novo para o Leaderboard
                if primeira || novas > 0 {
                    self.inicio_dados = self.an.agora;
                }
                if primeira {
                    let n = fonte.dados(&self.dados).jogos.len();
                    self.mostrar_toast(format!("{} conectada · {n} jogos", fonte.nome()), false);
                } else if manual {
                    self.mostrar_toast(format!("{} atualizada", fonte.nome()), false);
                }
            }
            Ok(None) => {}
            Err(e) => {
                if manual {
                    self.mostrar_toast(e.clone(), true);
                }
                self.externos[i].erro = Some(e);
            }
        }
    }

    /// Esquece a Steam/PSN e tira os jogos dela do app.
    pub(crate) fn desconectar_fonte(&mut self, fonte: Fonte) {
        self.plat.fonte_desconectar(fonte);
        crate::steam::remover(&mut self.dados, fonte);
        self.externos[fonte as usize] = Externo::default();
        self.externos[fonte as usize].estado.estado = "desligado".into();
        self.recalcular_jogos_iguais();
        self.salvar();
        self.ultima_publicacao = None;
        self.inicio_dados = self.an.agora;
        self.mostrar_toast(format!("{} desconectada", fonte.nome()), false);
    }

    pub(crate) fn salvar(&mut self) {
        if let Err(e) = self.dados.salvar(&self.plat.pasta_dados()) {
            self.mostrar_toast(format!("Não consegui salvar: {e}"), true);
        }
    }

    // ---------- Consultas ----------

    pub(crate) fn nome(&self, pacote: &str) -> String {
        self.apps
            .get(pacote)
            .map(|a| a.nome.clone())
            .or_else(|| self.dados.nomes.get(pacote).cloned())
            .unwrap_or_else(|| pacote.to_string())
    }

    /// Junta pacotes com o mesmo nome (o mesmo jogo vindo de lojas diferentes).
    pub(crate) fn recalcular_jogos_iguais(&mut self) {
        let mut grupos: HashMap<String, Vec<String>> = HashMap::new();
        let mut nomes: HashMap<String, String> = self.dados.nomes.clone().into_iter().collect();
        for (p, a) in &self.apps {
            nomes.insert(p.clone(), a.nome.clone());
        }
        for (p, n) in &nomes {
            if p == &self.proprio {
                continue;
            }
            grupos
                .entry(crate::ranking::chave_jogo(n, p))
                .or_default()
                .push(p.clone());
        }
        self.canon.clear();
        self.apelidos.clear();
        for (_, mut pacotes) in grupos {
            if pacotes.len() < 2 {
                continue;
            }
            // o principal: o que está instalado; depois o nome de pacote mais curto
            pacotes.sort_by_key(|p| (!self.apps.get(p).is_some_and(|a| a.abrivel), p.len(), p.clone()));
            let principal = pacotes[0].clone();
            for p in &pacotes {
                self.canon.insert(p.clone(), principal.clone());
            }
            self.apelidos.insert(principal, pacotes);
        }
    }

    /// O pacote "principal" de um jogo (o mesmo pacote, se não tiver repetido).
    pub(crate) fn canonico<'a>(&'a self, pacote: &'a str) -> &'a str {
        self.canon.get(pacote).map(|s| s.as_str()).unwrap_or(pacote)
    }

    /// Todos os pacotes do mesmo jogo.
    pub(crate) fn pacotes_de(&self, principal: &str) -> Vec<String> {
        self.apelidos
            .get(principal)
            .cloned()
            .unwrap_or_else(|| vec![principal.to_string()])
    }

    /// Conta como jogo se qualquer uma das versões contar.
    pub(crate) fn eh_jogo(&self, principal: &str) -> bool {
        self.pacotes_de(principal).iter().any(|p| self.dados.eh_jogo(p))
    }

    /// Marca/desmarca como jogo todas as versões juntas.
    pub(crate) fn definir_jogo(&mut self, principal: &str, jogo: bool) {
        for p in self.pacotes_de(principal) {
            self.dados.definir_jogo(&p, jogo);
        }
    }

    pub(crate) fn visivel(&self, pacote: &str) -> bool {
        let c = self.canonico(pacote);
        c != self.proprio && (self.dados.mostrar_todos || self.eh_jogo(c))
    }

    /// Todas as sessões que aparecem na tela (inclui a que está aberta agora),
    /// já com os jogos repetidos juntados.
    pub(crate) fn sessoes_visiveis(&self) -> Vec<Sessao> {
        let mut v: Vec<Sessao> = self
            .dados
            .sessoes
            .iter()
            .filter(|s| self.visivel(&s.pacote))
            .map(|s| Sessao {
                pacote: self.canonico(&s.pacote).to_string(),
                inicio: s.inicio,
                fim: s.fim,
            })
            .collect();
        if let Some((p, ini)) = &self.aberta {
            if self.visivel(p) {
                v.push(Sessao {
                    pacote: self.canonico(p).to_string(),
                    inicio: *ini,
                    fim: agora_ms(),
                });
            }
        }
        v
    }

    pub(crate) fn sessoes_de(&self, pacote: &str) -> Vec<Sessao> {
        let mut v: Vec<Sessao> = self
            .dados
            .sessoes
            .iter()
            .filter(|s| self.canonico(&s.pacote) == pacote)
            .map(|s| Sessao {
                pacote: pacote.to_string(),
                inicio: s.inicio,
                fim: s.fim,
            })
            .collect();
        if let Some((p, ini)) = &self.aberta {
            if self.canonico(p) == pacote {
                v.push(Sessao {
                    pacote: pacote.to_string(),
                    inicio: *ini,
                    fim: agora_ms(),
                });
            }
        }
        v
    }

    /// Tempo antigo da Steam/PSN (antes de conectar) dos jogos visíveis:
    /// (pacote principal, ms, última vez).
    pub(crate) fn bases_externas(&self) -> Vec<(String, i64, i64)> {
        let mut v = Vec::new();
        for f in Fonte::TODAS {
            for (id, j) in &f.dados(&self.dados).jogos {
                let p = f.pacote(id);
                if self.visivel(&p) {
                    v.push((self.canonico(&p).to_string(), j.base_ms, j.ultima_ms));
                }
            }
        }
        v
    }

    /// Tempo antigo (Steam/PSN) e última vez jogado de um jogo (somando as versões dele).
    pub(crate) fn base_externa_de(&self, principal: &str) -> (i64, i64) {
        let mut r = (0, 0);
        for p in self.pacotes_de(principal) {
            let Some(f) = Fonte::de_pacote(&p) else { continue };
            let Some(id) = f.id_de(&p) else { continue };
            if let Some(j) = f.dados(&self.dados).jogos.get(id) {
                r.0 += j.base_ms;
                r.1 = r.1.max(j.ultima_ms);
            }
        }
        r
    }

    /// Onde o jogo existe: "mobile" (algum pacote do celular), "steam", "psn".
    pub(crate) fn plataformas_de(&self, principal: &str) -> Vec<&'static str> {
        let pacotes = self.pacotes_de(principal);
        let mut v = Vec::new();
        if pacotes.iter().any(|p| !crate::steam::eh_externo(p)) {
            v.push("mobile");
        }
        for f in Fonte::TODAS {
            if pacotes.iter().any(|p| Fonte::de_pacote(p) == Some(f)) {
                v.push(f.chave());
            }
        }
        v
    }

    pub(crate) fn totais(&self, inicio: i64, fim: i64) -> Vec<Linha> {
        let mut m: HashMap<String, Linha> = HashMap::new();
        for s in self.sessoes_visiveis() {
            let t = sessoes::tempo_no_intervalo(&s, inicio, fim);
            let e = m.entry(s.pacote.clone()).or_insert(Linha {
                pacote: s.pacote.clone(),
                tempo: 0,
                ultima: 0,
                sessoes: 0,
            });
            e.tempo += t;
            e.ultima = e.ultima.max(s.fim);
            if t > 0 {
                e.sessoes += 1;
            }
        }
        // o tempo de antes de conectar a Steam/PSN só conta em "Tudo"
        for (p, base, ultima) in self.bases_externas() {
            if inicio > 0 && !m.contains_key(&p) {
                continue;
            }
            let e = m.entry(p.clone()).or_insert(Linha {
                pacote: p,
                tempo: 0,
                ultima: 0,
                sessoes: 0,
            });
            if inicio <= 0 {
                e.tempo += base;
            }
            e.ultima = e.ultima.max(ultima);
        }
        let mut v: Vec<Linha> = m.into_values().filter(|l| l.tempo > 0).collect();
        // Desempate fixo (quem jogou por último, depois o nome do pacote): sem isso, jogos com o
        // mesmo tempo trocavam de lugar a cada quadro, porque a ordem do HashMap muda.
        v.sort_by(|a, b| {
            b.tempo
                .cmp(&a.tempo)
                .then(b.ultima.cmp(&a.ultima))
                .then(a.pacote.cmp(&b.pacote))
        });
        v
    }

    /// Tempo por dia nos últimos 7 dias: (início do dia, tempo, nº de jogos).
    pub(crate) fn por_dia(sess: &[Sessao]) -> Vec<(i64, i64, usize)> {
        let agora = agora_ms();
        (0..7)
            .rev()
            .map(|d| {
                let ini = inicio_do_dia(d);
                let fim = if d == 0 { agora } else { inicio_do_dia(d - 1) };
                let mut jogos = std::collections::BTreeSet::new();
                let mut total = 0;
                for s in sess {
                    let t = sessoes::tempo_no_intervalo(s, ini, fim);
                    if t > 0 {
                        total += t;
                        jogos.insert(s.pacote.as_str());
                    }
                }
                (ini, total, jogos.len())
            })
            .collect()
    }

    // ---------- Coisas do sistema (tema, barras) ----------

    fn checar_sistema(&mut self, ctx: &egui::Context) {
        let vencido = |t: Option<Instant>, ms: u64| t.map_or(true, |t| t.elapsed() > Duration::from_millis(ms));

        // conta: mais rápido enquanto está entrando
        let intervalo = if self.conta.estado == EstadoConta::Entrando {
            400
        } else {
            1500
        };
        if vencido(self.ultimo_check_conta, intervalo) {
            let nova = self.plat.conta();
            // mudou o nome ou a foto: envia de novo e atualiza o ranking
            if nova.estado == EstadoConta::Logado
                && self.conta.estado == EstadoConta::Logado
                && (nova.nome != self.conta.nome || nova.foto != self.conta.foto)
            {
                self.ultima_publicacao = None;
                self.lb_pedido = None;
            }
            if nova.estado == EstadoConta::Logado && self.conta.estado != EstadoConta::Logado {
                self.ultima_publicacao = None; // publica logo que entra
                if self.conta.estado == EstadoConta::Entrando {
                    self.mostrar_toast(format!("Olá, {}!", nova.nome.split(' ').next().unwrap_or("")), false);
                }
            }
            if nova.estado == EstadoConta::Erro && self.conta.estado == EstadoConta::Entrando {
                self.mostrar_toast(nova.msg.clone(), true);
            }
            self.conta = nova;
            self.ultimo_check_conta = Some(Instant::now());
        }
        if let Some(rx) = &self.rx_apagar {
            if let Ok(r) = rx.try_recv() {
                self.rx_apagar = None;
                self.ultimo_check_conta = None;
                self.lb_cache.clear();
                leaderboard::apagar_cache(&self.plat.pasta_dados());
                match r {
                    Ok(()) => self.mostrar_toast("Seus dados saíram do Leaderboard", false),
                    Err(e) => self.mostrar_toast(format!("Não consegui apagar: {e}"), true),
                }
            }
        }
        // Steam e PSN: mais rápido enquanto está conectando
        for f in Fonte::TODAS {
            let ex = &mut self.externos[f as usize];
            let intervalo = if ex.estado.estado == "conectando" { 500 } else { 2000 };
            if vencido(ex.ultimo_check, intervalo) {
                let novo = self.plat.fonte_estado(f);
                if novo.conectado() && !ex.estado.conectado() {
                    ex.ultima_sync = None; // acabou de conectar: lê os jogos já
                }
                ex.estado = novo;
                ex.ultimo_check = Some(Instant::now());
            }
        }

        if self.imagens.receber(ctx) {
            self.an.ativo = true;
        }

        if !self.permissao && self.ultimo_check.elapsed() > Duration::from_millis(1500) {
            self.permissao = self.plat.tem_permissao();
            self.ultimo_check = Instant::now();
            if self.permissao {
                self.trocar_pagina(0.0);
            }
        }
        if vencido(self.ultimo_check_tema, 2000) {
            self.escuro_sistema = self.plat.tema_escuro_sistema();
            self.ultimo_check_tema = Some(Instant::now());
        }
        if self.margens.is_none() || vencido(self.ultimo_check_margens, 3000) {
            if let Some(m) = self.plat.margens_sistema() {
                self.margens = Some(m);
            }
            self.ultimo_check_margens = Some(Instant::now());
        }

        let escuro_alvo = match self.dados.tema {
            1 => false,
            2 => true,
            _ => self
                .escuro_sistema
                .or_else(|| ctx.system_theme().map(|t| t == egui::Theme::Dark))
                .unwrap_or(true),
        };
        self.escuro = self.an.ir("tema", if escuro_alvo { 1.0 } else { 0.0 }, 0.4);
        self.p = Paleta::misturar(self.escuro);
        if (self.escuro - self.estilo_aplicado).abs() > 0.002 {
            tema::aplicar_estilo(ctx, &self.p, self.escuro > 0.5);
            self.estilo_aplicado = self.escuro;
        }
    }
}

impl eframe::App for AbyssHubApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let agora = ctx.input(|i| i.time);
        self.an.quadro(agora);
        // Ficou mais de 2 minutos sem desenhar = o app estava em segundo plano e foi aberto de novo:
        // conta como "abrir o app" (procura atualização de novo).
        if self
            .ultimo_quadro
            .is_some_and(|t| t.elapsed() > Duration::from_secs(120))
        {
            self.upd_tentou = false;
            self.boot = Some(agora - 3.0);
        }
        self.ultimo_quadro = Some(Instant::now());
        self.an.desligado = !self.dados.animacoes;
        if self.boot.is_none() {
            self.boot = Some(agora);
            self.inicio_pagina = agora;
            self.inicio_dados = agora;
        }

        self.receber();
        self.checar_sistema(ctx);

        // Tecla Esc (no PC): fecha o painel ou o detalhe.
        let voltar = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if voltar {
            if self.upd_aberto && self.upd_baixa.estado != "baixando" {
                self.upd_aberto = false;
            } else if self.ajustes_aberto {
                self.ajustes_aberto = false;
            } else if self.detalhe.is_some() {
                self.fechar_detalhe();
            }
        }

        if self.permissao
            && self.rx.is_none()
            && self.ultima_sync.map_or(true, |t| t.elapsed() > Duration::from_secs(30))
        {
            self.sincronizar();
        }
        // Steam e PSN: lê os jogos ao conectar e depois a cada 10 minutos com o app aberto
        for f in Fonte::TODAS {
            self.receber_fonte(f);
            let ex = &self.externos[f as usize];
            if ex.estado.conectado()
                && ex.rx.is_none()
                && ex.ultima_sync.map_or(true, |t| t.elapsed() > Duration::from_secs(600))
            {
                self.sincronizar_fonte(f, false);
            }
        }
        if self.permissao {
            self.cuidar_leaderboard();
            self.cuidar_placar_e_eventos();
        }
        self.cuidar_atualizacao();

        let ppp = ctx.pixels_per_point().max(0.5);
        let (topo_px, baixo_px) = self.margens.unwrap_or((0.0, 0.0));
        let (topo, baixo) = (topo_px / ppp, baixo_px / ppp);

        if self.permissao {
            egui::TopBottomPanel::bottom("abas")
                .frame(egui::Frame::default().fill(self.p.card))
                .show_separator_line(false)
                .exact_height(66.0 + baixo)
                .show(ctx, |ui| self.barra_abas(ui));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(self.p.bg))
            .show(ctx, |ui| self.pagina(ui, topo));
        if self.permissao {
            self.botao_flutuante(ctx, baixo);
        }
        self.folha_ajustes(ctx, topo);
        self.folha_atualizacao(ctx);
        self.desenhar_toast(ctx, baixo);

        let esperando = self.rx_hist.is_some() || self.rx_ev.is_some() || self.rx_pl_tudo.is_some() || self.pl_esperando_data;
        if self.an.ativo || self.rx.is_some() || self.externos.iter().any(|e| e.rx.is_some()) || esperando {
            ctx.request_repaint();
        } else if self.dados.animacoes {
            // O brilho do topo se mexe devagar: ~20 quadros por segundo bastam e poupam bateria.
            ctx.request_repaint_after(Duration::from_millis(50));
        } else {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

// ---------- Datas e durações ----------

const DIAS_SEMANA: [&str; 7] = ["seg", "ter", "qua", "qui", "sex", "sáb", "dom"];
const MESES: [&str; 12] = [
    "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
];

/// Meia-noite (horário local) de `dias_atras` dias atrás, em ms.
pub(crate) fn inicio_do_dia(dias_atras: i64) -> i64 {
    let hoje = Local::now().date_naive();
    let dia = if dias_atras >= 0 {
        hoje.checked_sub_days(Days::new(dias_atras as u64))
    } else {
        hoje.checked_add_days(Days::new((-dias_atras) as u64))
    }
    .unwrap_or(hoje);
    dia.and_hms_opt(0, 0, 0)
        .and_then(|d| d.and_local_timezone(Local).earliest())
        .map(|d| d.timestamp_millis())
        .unwrap_or(0)
}

pub(crate) fn fmt_duracao(ms: i64) -> String {
    let s = (ms / 1000).max(0);
    let (h, m) = (s / 3600, (s % 3600) / 60);
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m")
    } else {
        format!("{s}s")
    }
}

pub(crate) fn fmt_curto(ms: i64) -> String {
    let m = ms / 60_000;
    if m >= 60 {
        let h = m as f64 / 60.0;
        if h >= 10.0 {
            format!("{h:.0}h")
        } else {
            format!("{h:.1}h").replace('.', ",")
        }
    } else {
        format!("{m}m")
    }
}

pub(crate) fn fmt_hora(ms: i64) -> String {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_default()
}

pub(crate) fn fmt_dia_curto(ms: i64) -> String {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| d.format("%d").to_string())
        .unwrap_or_default()
}

/// "hoje", "ontem" ou "sex, 14 de nov".
pub(crate) fn fmt_dia(ms: i64) -> String {
    if ms >= inicio_do_dia(0) {
        return "Hoje".into();
    }
    if ms >= inicio_do_dia(1) {
        return "Ontem".into();
    }
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map(|d| {
            let sem = DIAS_SEMANA[d.weekday().num_days_from_monday() as usize];
            let mes = MESES[d.month0() as usize];
            let mut s = format!("{sem}, {} de {mes}", d.day());
            if let Some(c) = s.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            s
        })
        .unwrap_or_default()
}

pub(crate) fn fmt_relativo(ms: i64) -> String {
    if ms <= 0 {
        return "-".into();
    }
    let min = (agora_ms() - ms) / 60_000;
    if min < 1 {
        "agora".into()
    } else if min < 60 {
        format!("há {min} min")
    } else if min < 60 * 24 {
        format!("há {} h", min / 60)
    } else if min < 60 * 24 * 2 {
        "ontem".into()
    } else {
        format!("há {} dias", min / (60 * 24))
    }
}

pub(crate) fn cor_categoria(i: usize) -> Color32 {
    // As mesmas cores das categorias do AbyssSub.
    const CORES: [Color32; 7] = [
        Color32::from_rgb(0x2f, 0x80, 0xed),
        Color32::from_rgb(0x34, 0xa8, 0x53),
        Color32::from_rgb(0x7b, 0x4b, 0xe6),
        Color32::from_rgb(0xe8, 0x59, 0x0c),
        Color32::from_rgb(0xb4, 0x3f, 0xd1),
        Color32::from_rgb(0xe8, 0x4a, 0x8a),
        Color32::from_rgb(0xe3, 0xad, 0x58),
    ];
    CORES[i % CORES.len()]
}
