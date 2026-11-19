//! Tudo que depende do sistema (Android de verdade ou PC com dados de teste).

pub use crate::steam::Fonte;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::sessoes::Evento;

#[cfg(target_os = "android")]
pub mod android;
#[cfg(not(target_os = "android"))]
pub mod desktop;

#[derive(Clone, Debug)]
pub struct InfoApp {
    pub pacote: String,
    pub nome: String,
    /// O Android classifica o app como jogo (categoria "Jogo" ou flag isGame).
    pub jogo: bool,
    /// Tem ícone na gaveta de apps (dá para abrir).
    pub abrivel: bool,
}

/// Situação do login com Google.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum EstadoConta {
    /// O app foi gerado sem Firebase (falta o google-services.json) ou é a versão PC.
    #[default]
    Indisponivel,
    Saiu,
    Entrando,
    Logado,
    Erro,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Conta {
    pub estado: EstadoConta,
    pub msg: String,
    pub uid: String,
    pub nome: String,
    pub foto: String,
    pub email: String,
    /// Formas de entrar ligadas à conta: "google.com", "password".
    pub provedores: Vec<String>,
}

/// Imagens que o app pede ao sistema (sempre quadradas, RGBA pré-multiplicado).
#[derive(Clone, Debug)]
pub enum PedidoImagem {
    /// Ícone de um app (do celular ou, se não estiver instalado, do Firebase).
    Icone(String),
    /// Foto de uma pessoa (endereço da internet).
    Foto(String),
}

pub trait Plataforma: Send + Sync {
    /// O usuário já liberou o "Acesso ao uso"?
    fn tem_permissao(&self) -> bool;
    /// Abre a tela do Android onde se libera o "Acesso ao uso".
    fn abrir_permissao(&self);
    /// Apps instalados.
    fn listar_apps(&self) -> Result<Vec<InfoApp>, String>;
    /// Eventos de abrir/fechar apps entre `desde` e `ate` (ms desde 1970).
    fn eventos(&self, desde: i64, ate: i64) -> Result<Vec<Evento>, String>;
    /// Pasta onde o app salva os próprios dados.
    fn pasta_dados(&self) -> PathBuf;
    /// Nome do pacote do próprio AbyssHub (para não contar ele mesmo).
    fn pacote_proprio(&self) -> String;
    /// O sistema está no modo escuro? (None = não sei)
    fn tema_escuro_sistema(&self) -> Option<bool> {
        None
    }
    /// Altura (em pixels) da barra de status em cima e da barra de navegação embaixo.
    fn margens_sistema(&self) -> Option<(f32, f32)> {
        Some((0.0, 0.0))
    }

    // ---------- Conta e Leaderboard ----------

    fn conta(&self) -> Conta {
        Conta::default()
    }
    /// Começa o login com Google (o resultado aparece depois em `conta()`).
    fn entrar(&self) {}
    fn sair(&self) {}
    /// Envia o tempo de jogo da pessoa (JSON montado em ranking.rs).
    fn publicar(&self, _json: &str) -> Result<(), String> {
        Err("Leaderboard indisponível".into())
    }
    /// Busca o ranking. `chave` é a chave do jogo (modo "jogo"). Devolve uma lista em JSON.
    fn buscar_ranking(
        &self,
        _modo: &str,
        _periodo: &str,
        _plataforma: &str,
        _chave: &str,
        _dia: &str,
    ) -> Result<String, String> {
        Err("Leaderboard indisponível".into())
    }
    fn apagar_meus_dados(&self) -> Result<(), String> {
        Err("Leaderboard indisponível".into())
    }
    /// Pixels RGBA (pré-multiplicados) de `tam` x `tam`.
    fn imagem(&self, _pedido: &PedidoImagem, _tam: u32) -> Option<Vec<u8>> {
        None
    }
    /// Abre um link no navegador.
    fn abrir_link(&self, _url: &str) {}
    /// Abre a janelinha de entrar/criar conta com e-mail e senha.
    fn entrar_email(&self) {}
    /// Mudar o nome que aparece no Leaderboard.
    fn editar_nome(&self) {}
    /// Mudar a foto que aparece no Leaderboard (galeria, a do Google ou nenhuma).
    fn editar_foto(&self) {}
    /// Conectar uma conta Google à conta atual.
    fn ligar_google(&self) {}
    /// Adicionar e-mail e senha à conta atual.
    fn ligar_email(&self) {}

    // ---------- Steam e PlayStation ----------

    /// Situação da conexão com a Steam ou a PSN.
    fn fonte_estado(&self, _fonte: Fonte) -> EstadoFonte {
        EstadoFonte::default()
    }
    /// Abre a janelinha para conectar (Steam: perfil e chave; PSN: ID online e código).
    fn fonte_conectar(&self, _fonte: Fonte) {}
    /// Esquece a conta.
    fn fonte_desconectar(&self, _fonte: Fonte) {}
    /// Lê os jogos (JSON {"id", "jogos": [...]}). None = não conectada.
    fn fonte_jogos(&self, _fonte: Fonte) -> Result<Option<String>, String> {
        Ok(None)
    }

    // ---------- Atualização automática (GitHub Releases) ----------

    /// JSON da última versão publicada, ou None se ainda não há versões.
    fn verificar_atualizacao(&self, _repo: &str) -> Result<Option<String>, String> {
        Err("A verificação de atualizações funciona no app Android".into())
    }
    /// Começa a baixar o APK (o andamento aparece em `estado_atualizacao`).
    fn baixar_atualizacao(&self, _url: &str) {}
    /// Abre o instalador do Android com o APK baixado.
    fn instalar_atualizacao(&self) {}
    /// JSON com {"estado", "progresso", "msg"}.
    fn estado_atualizacao(&self) -> String {
        String::new()
    }
}

/// Situação da Steam ou da PSN: "indisponivel", "desligado", "conectando", "pendente" (PSN: falta
/// verificar o código), "ok" ou "erro".
#[derive(Clone, Debug, PartialEq)]
pub struct EstadoFonte {
    pub estado: String,
    pub msg: String,
    /// Nome do perfil (Steam) ou ID online (PSN).
    pub nome: String,
}

impl Default for EstadoFonte {
    fn default() -> Self {
        EstadoFonte {
            estado: "indisponivel".into(),
            msg: String::new(),
            nome: String::new(),
        }
    }
}

impl EstadoFonte {
    pub fn conectado(&self) -> bool {
        self.estado == "ok"
    }
}

/// Lê o JSON de steamEstado() / psnEstado() (MainActivity.java).
#[allow(dead_code)]
pub fn ler_fonte(json: &str) -> EstadoFonte {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    let t = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let estado = t("estado");
    EstadoFonte {
        estado: if estado.is_empty() { "indisponivel".into() } else { estado },
        msg: t("msg"),
        nome: t("nome"),
    }
}

/// Lê o JSON de estadoConta() (MainActivity.java).
#[allow(dead_code)]
pub fn ler_conta(json: &str) -> Conta {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return Conta::default(),
    };
    let t = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let estado = match t("estado").as_str() {
        "logado" => EstadoConta::Logado,
        "entrando" => EstadoConta::Entrando,
        "erro" => EstadoConta::Erro,
        "saiu" => EstadoConta::Saiu,
        _ => EstadoConta::Indisponivel,
    };
    let provedores = v
        .get("provedores")
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|p| p.as_str().map(String::from)).collect())
        .unwrap_or_default();
    Conta {
        estado,
        msg: t("msg"),
        uid: t("uid"),
        nome: t("nome"),
        foto: t("foto"),
        email: t("email"),
        provedores,
    }
}

pub fn agora_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
