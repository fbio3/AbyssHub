//! Ícones de jogos e fotos de pessoas: carrega numa thread separada,
//! guarda no celular (pasta "imagens") e vira textura do egui.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, TextureHandle, TextureId, TextureOptions};

use crate::plataforma::{PedidoImagem, Plataforma};

/// Tamanho (em pixels) de todas as imagens guardadas.
pub const TAM: u32 = 96;

enum Estado {
    Carregando,
    Pronta(TextureHandle),
    Falhou(Instant),
}

pub struct Imagens {
    mapa: HashMap<String, Estado>,
    tx: Sender<(String, PedidoImagem)>,
    rx: Receiver<(String, Option<Vec<u8>>)>,
}

fn nome_arquivo(chave: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in chave.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}.rgba")
}

impl Imagens {
    pub fn nova(plat: Arc<dyn Plataforma>, pasta: PathBuf) -> Self {
        let (tx, rx_pedidos) = mpsc::channel::<(String, PedidoImagem)>();
        let (tx_res, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = fs::create_dir_all(&pasta);
            let esperado = (TAM * TAM * 4) as usize;
            for (chave, pedido) in rx_pedidos {
                let arq = pasta.join(nome_arquivo(&chave));
                let bytes = match fs::read(&arq) {
                    Ok(b) if b.len() == esperado => Some(b),
                    _ => {
                        let b = plat.imagem(&pedido, TAM);
                        if let Some(b) = &b {
                            let _ = fs::write(&arq, b);
                        }
                        b
                    }
                };
                if tx_res.send((chave, bytes)).is_err() {
                    break;
                }
            }
        });
        Imagens {
            mapa: HashMap::new(),
            tx,
            rx,
        }
    }

    /// Transforma em textura o que a thread terminou de carregar. Devolve true se chegou algo.
    pub fn receber(&mut self, ctx: &egui::Context) -> bool {
        let mut chegou = false;
        while let Ok((chave, bytes)) = self.rx.try_recv() {
            chegou = true;
            let estado = match bytes {
                Some(b) => {
                    let img = ColorImage::from_rgba_premultiplied([TAM as usize, TAM as usize], &b);
                    Estado::Pronta(ctx.load_texture(chave.clone(), img, TextureOptions::LINEAR))
                }
                None => Estado::Falhou(Instant::now()),
            };
            self.mapa.insert(chave, estado);
        }
        chegou
    }

    fn pegar(&mut self, chave: String, pedido: impl FnOnce() -> PedidoImagem) -> Option<TextureId> {
        match self.mapa.get(&chave) {
            Some(Estado::Pronta(t)) => return Some(t.id()),
            Some(Estado::Carregando) => return None,
            Some(Estado::Falhou(t)) if t.elapsed() < Duration::from_secs(300) => return None,
            _ => {}
        }
        self.mapa.insert(chave.clone(), Estado::Carregando);
        let _ = self.tx.send((chave, pedido()));
        None
    }

    pub fn icone(&mut self, pacote: &str) -> Option<TextureId> {
        if pacote.is_empty() {
            return None;
        }
        self.pegar(format!("icone:{pacote}"), || PedidoImagem::Icone(pacote.to_string()))
    }

    pub fn foto(&mut self, url: &str) -> Option<TextureId> {
        if url.is_empty() {
            return None;
        }
        self.pegar(format!("foto:{url}"), || PedidoImagem::Foto(url.to_string()))
    }
}
