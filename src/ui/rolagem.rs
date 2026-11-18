//! Rolagem com impulso: ao soltar o dedo a página continua rolando
//! conforme a força do movimento e vai parando aos poucos (como nos apps do Android/iPhone).
//!
//! A roda do mouse e a barra de rolagem continuam funcionando normalmente.

use std::collections::VecDeque;

use eframe::egui::{self, Rect, Ui};

use super::AbyssHubApp;

/// Quanto a velocidade diminui por segundo (maior = para mais rápido).
const ATRITO: f32 = 2.6;
/// Velocidade máxima (pontos por segundo).
const VEL_MAX: f32 = 7000.0;
/// Abaixo disso a rolagem para.
const VEL_MIN: f32 = 12.0;
/// Quanto o dedo precisa andar antes de decidir se é rolagem (vertical) ou outra coisa.
const LIMIAR: f32 = 8.0;

#[derive(Default)]
pub(crate) struct Rolagem {
    offset: f32,
    vel: f32,
    arrastando: bool,
    /// Some(true) = o dedo está rolando na vertical; Some(false) = movimento de lado (ignora)
    vertical: Option<bool>,
    /// (tempo, posição) dos últimos instantes do arraste, para medir a força ao soltar
    amostras: VecDeque<(f64, f32)>,
    area: Option<Rect>,
    max: f32,
}

impl AbyssHubApp {
    /// Igual a um `ScrollArea::vertical`, mas com impulso. `chave` separa a posição de cada tela.
    pub(super) fn rolagem(&mut self, ui: &mut Ui, chave: &str, add: impl FnOnce(&mut Self, &mut Ui)) {
        let mut st = self.rolagens.remove(chave).unwrap_or_default();
        let ctx = ui.ctx().clone();
        let agora = self.an.agora;
        let dt = ctx.input(|i| i.stable_dt).clamp(1.0 / 240.0, 0.05);
        let camada = ui.layer_id();
        let largura_barra = ui.spacing().scroll.bar_width + 10.0;

        let (apertado, origem, pos, delta) = ctx.input(|i| {
            (
                i.pointer.primary_down(),
                i.pointer.press_origin(),
                i.pointer.interact_pos(),
                i.pointer.delta(),
            )
        });

        // O toque começou dentro desta área (e não em cima de outra coisa, nem na barra)?
        let nosso = match (origem, st.area) {
            (Some(o), Some(a)) => {
                a.contains(o) && o.x < a.right() - largura_barra && ctx.layer_id_at(o) == Some(camada)
            }
            _ => false,
        };

        if apertado && nosso {
            if !st.arrastando {
                st.arrastando = true;
                st.vertical = None;
                st.vel = 0.0;
                st.amostras.clear();
            }
            if st.vertical.is_none() {
                if let (Some(o), Some(p)) = (origem, pos) {
                    let d = p - o;
                    if d.length() > LIMIAR {
                        st.vertical = Some(d.y.abs() >= d.x.abs());
                    }
                }
            }
            if st.vertical == Some(true) {
                st.offset -= delta.y;
                st.amostras.push_back((agora, st.offset));
                while st.amostras.front().is_some_and(|(t, _)| agora - *t > 0.1) {
                    st.amostras.pop_front();
                }
            }
        } else if st.arrastando {
            // soltou: a velocidade é o quanto andou nos últimos ~0,1 s
            st.arrastando = false;
            st.vel = 0.0;
            if st.vertical == Some(true) {
                if let (Some(a), Some(b)) = (st.amostras.front(), st.amostras.back()) {
                    let tempo = (b.0 - a.0) as f32;
                    let parado = agora - b.0 > 0.08; // segurou parado antes de soltar
                    if tempo > 0.01 && !parado {
                        st.vel = ((b.1 - a.1) / tempo).clamp(-VEL_MAX, VEL_MAX);
                    }
                }
            }
            st.amostras.clear();
        } else if st.vel != 0.0 {
            st.offset += st.vel * dt;
            st.vel *= (-ATRITO * dt).exp();
            if st.vel.abs() < VEL_MIN {
                st.vel = 0.0;
            }
        }

        // não passa do começo nem do fim
        let max = st.max.max(0.0);
        if st.offset < 0.0 || st.offset > max {
            st.offset = st.offset.clamp(0.0, max);
            if !st.arrastando {
                st.vel = 0.0;
            }
        }

        let pedido = st.offset;
        let saida = egui::ScrollArea::vertical()
            .id_salt(chave)
            .auto_shrink([false, false])
            .drag_to_scroll(false)
            .vertical_scroll_offset(pedido)
            .show(ui, |ui| add(self, ui));

        // Se a roda do mouse ou a barra mexeram, segue a posição do egui.
        let real = saida.state.offset.y;
        if (real - pedido).abs() > 0.5 && !(st.arrastando && st.vertical == Some(true)) {
            st.offset = real;
            st.vel = 0.0;
        }
        st.max = (saida.content_size.y - saida.inner_rect.height()).max(0.0);
        st.area = Some(saida.inner_rect);

        if st.vel != 0.0 || st.arrastando {
            self.an.ativo = true;
        }
        self.rolagens.insert(chave.to_string(), st);
    }
}
