//! O que o AbyssHub guarda no celular (arquivo JSON na pasta interna do app).
//!
//! O Android só mantém o histórico de uso por alguns dias, então o app copia as
//! sessões para cá a cada sincronização e o tempo total nunca se perde.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::sessoes::Sessao;

const ARQUIVO: &str = "abysshub.json";

#[derive(Serialize, Deserialize)]
pub struct Dados {
    #[serde(default)]
    pub versao: u32,
    /// Todas as sessões registradas, ordenadas pelo início.
    #[serde(default)]
    pub sessoes: Vec<Sessao>,
    /// Até onde (em ms) o histórico do Android já foi lido.
    #[serde(default)]
    pub sincronizado_ate: i64,
    /// Nome dos apps que têm sessão (para continuar aparecendo se forem desinstalados).
    #[serde(default)]
    pub nomes: BTreeMap<String, String>,
    /// Apps que o próprio Android diz que são jogos.
    #[serde(default)]
    pub auto_jogo: BTreeSet<String>,
    /// Apps que o usuário marcou como jogo.
    #[serde(default)]
    pub marcados_jogo: BTreeSet<String>,
    /// Apps que o Android diz que são jogo mas o usuário desmarcou.
    #[serde(default)]
    pub desmarcados_jogo: BTreeSet<String>,
    /// Mostrar também apps que não são jogos.
    #[serde(default)]
    pub mostrar_todos: bool,
    /// 0 = seguir o sistema, 1 = claro, 2 = escuro.
    #[serde(default)]
    pub tema: u8,
    /// Animações ligadas (o brilho do topo, cartões entrando, gráficos crescendo...).
    #[serde(default = "verdadeiro")]
    pub animacoes: bool,
    /// Atualização automática (última versão vista, "agora não"...).
    #[serde(default)]
    pub atualizacao: crate::atualizacao::EstadoSalvo,
}

fn verdadeiro() -> bool {
    true
}

impl Default for Dados {
    fn default() -> Self {
        Dados {
            versao: 1,
            sessoes: Vec::new(),
            sincronizado_ate: 0,
            nomes: BTreeMap::new(),
            auto_jogo: BTreeSet::new(),
            marcados_jogo: BTreeSet::new(),
            desmarcados_jogo: BTreeSet::new(),
            mostrar_todos: false,
            tema: 0,
            animacoes: true,
            atualizacao: Default::default(),
        }
    }
}

impl Dados {
    pub fn carregar(pasta: &Path) -> Dados {
        fs::read_to_string(pasta.join(ARQUIVO))
            .ok()
            .and_then(|txt| serde_json::from_str(&txt).ok())
            .unwrap_or_default()
    }

    pub fn salvar(&self, pasta: &Path) -> io::Result<()> {
        fs::create_dir_all(pasta)?;
        let txt = serde_json::to_string(self).map_err(io::Error::other)?;
        // Escreve num arquivo temporário e troca, para não corromper se o app fechar no meio.
        let tmp = pasta.join(format!("{ARQUIVO}.tmp"));
        fs::write(&tmp, txt)?;
        fs::rename(&tmp, pasta.join(ARQUIVO))
    }

    pub fn eh_jogo(&self, pacote: &str) -> bool {
        if self.desmarcados_jogo.contains(pacote) {
            return false;
        }
        self.marcados_jogo.contains(pacote) || self.auto_jogo.contains(pacote)
    }

    pub fn definir_jogo(&mut self, pacote: &str, jogo: bool) {
        let auto = self.auto_jogo.contains(pacote);
        if jogo {
            self.desmarcados_jogo.remove(pacote);
            if !auto {
                self.marcados_jogo.insert(pacote.to_string());
            }
        } else {
            self.marcados_jogo.remove(pacote);
            if auto {
                self.desmarcados_jogo.insert(pacote.to_string());
            }
        }
    }
}
