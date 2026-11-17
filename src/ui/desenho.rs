//! Desenhos feitos à mão: degradê do topo, brilho, rosca, ícones e
//! componentes animados (interruptor, seletor com "bolinha" deslizante).

use std::f32::consts::{PI, TAU};

use eframe::egui::{
    self, epaint::Mesh, epaint::Vertex, pos2, vec2, Align2, Color32, CornerRadius, Painter, Pos2, Rect, Response,
    Sense, Shape, Stroke, TextureId, Ui, Vec2,
};

use super::anim::Animador;
use super::tema::{alfa, fonte, misturar_cor, Paleta, Peso};

// ---------- Formas ----------

/// Contorno de um retângulo com só os cantos de baixo arredondados.
fn contorno_topo(rect: Rect, raio: f32) -> Vec<Pos2> {
    let r = raio.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let mut pts = vec![rect.left_top(), rect.right_top()];
    let passos = 10;
    // canto inferior direito
    let c = pos2(rect.right() - r, rect.bottom() - r);
    for i in 0..=passos {
        let a = (i as f32 / passos as f32) * (PI / 2.0);
        pts.push(c + vec2(a.cos(), a.sin()) * r);
    }
    // canto inferior esquerdo
    let c = pos2(rect.left() + r, rect.bottom() - r);
    for i in 0..=passos {
        let a = PI / 2.0 + (i as f32 / passos as f32) * (PI / 2.0);
        pts.push(c + vec2(a.cos(), a.sin()) * r);
    }
    pts
}

/// Degradê linear igual ao `linear-gradient(155deg, a, b)` do CSS,
/// com os cantos de baixo arredondados (o "hero" do AbyssSub).
pub fn degrade_topo(rect: Rect, raio_baixo: f32, a: Color32, b: Color32) -> Shape {
    let ang = 155f32.to_radians();
    let dir = vec2(ang.sin(), -ang.cos());
    let meio = rect.center();
    let meia = (rect.width() * dir.x.abs() + rect.height() * dir.y.abs()) / 2.0;
    let cor = |p: Pos2| {
        let t = ((p - meio).dot(dir) / meia + 1.0) / 2.0;
        misturar_cor(a, b, t)
    };
    let pts = contorno_topo(rect, raio_baixo);
    let mut m = Mesh::default();
    m.colored_vertex(meio, cor(meio));
    for p in &pts {
        m.colored_vertex(*p, cor(*p));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    Shape::mesh(m)
}

/// Brilho redondo e suave (o `radial-gradient` que flutua no topo).
pub fn brilho(centro: Pos2, raio: f32, cor: Color32) -> Shape {
    let mut m = Mesh::default();
    let aneis: [(f32, f32); 3] = [(0.0, 1.0), (0.35, 0.6), (0.65, 0.0)];
    let passos = 48u32;
    m.colored_vertex(centro, cor);
    for &(fr, a) in aneis.iter().skip(1) {
        for i in 0..passos {
            let ang = i as f32 / passos as f32 * TAU;
            m.colored_vertex(centro + vec2(ang.cos(), ang.sin()) * raio * fr, alfa(cor, a));
        }
    }
    // centro -> anel 1
    for i in 0..passos {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % passos);
    }
    // anel 1 -> anel 2
    let (a1, a2) = (1u32, 1 + passos);
    for i in 0..passos {
        let j = (i + 1) % passos;
        m.add_triangle(a1 + i, a2 + i, a2 + j);
        m.add_triangle(a1 + i, a2 + j, a1 + j);
    }
    Shape::mesh(m)
}

/// Sombra suave embaixo de um círculo (o box-shadow do botão dourado).
pub fn sombra_circulo(p: &Painter, centro: Pos2, raio: f32, cor: Color32) {
    for i in 0..6 {
        let k = i as f32 / 6.0;
        p.circle_filled(
            centro + vec2(0.0, 6.0),
            raio + 14.0 * (1.0 - k),
            alfa(cor, 0.05 + 0.05 * k),
        );
    }
}

/// Rosca (gráfico de pizza com furo). `prog` 0..1 faz ela "se desenhar".
pub fn rosca(
    p: &Painter,
    centro: Pos2,
    raio: f32,
    espessura: f32,
    fatias: &[(f32, Color32)],
    prog: f32,
    fundo: Color32,
) {
    let r = raio - espessura / 2.0;
    p.circle_stroke(centro, r, Stroke::new(espessura, fundo));
    let total: f32 = fatias.iter().map(|f| f.0).sum::<f32>().max(1e-6);
    let limite = prog.clamp(0.0, 1.0) * TAU;
    let folga = if fatias.len() > 1 { 0.035 } else { 0.0 };
    let mut ang = -PI / 2.0;
    let mut usado = 0.0;
    for (valor, cor) in fatias {
        let varredura = valor / total * TAU;
        let ini = usado;
        let fim = (usado + varredura - folga).min(limite);
        usado += varredura;
        if fim <= ini {
            ang += varredura;
            continue;
        }
        let passos = ((fim - ini) / 0.05).ceil().max(2.0) as usize;
        let pts: Vec<Pos2> = (0..=passos)
            .map(|i| {
                let a = ang + (fim - ini) * i as f32 / passos as f32;
                centro + vec2(a.cos(), a.sin()) * r
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(espessura, *cor)));
        ang += varredura;
    }
}

// ---------- Ícones (traço de 2 px, como os SVG do AbyssSub) ----------

#[allow(dead_code)]
pub enum Icone {
    Casa,
    Controle,
    Calendario,
    Engrenagem,
    Atualizar,
    Voltar,
    Relogio,
    Trofeu,
}

pub fn icone(p: &Painter, qual: Icone, c: Pos2, tam: f32, cor: Color32, giro: f32) {
    let s = tam / 24.0;
    let st = Stroke::new(2.0 * s.max(0.8), cor);
    let rot = |x: f32, y: f32| {
        let (sn, cs) = giro.sin_cos();
        c + vec2(x * cs - y * sn, x * sn + y * cs) * s
    };
    let pt = |x: f32, y: f32| c + vec2(x, y) * s;
    match qual {
        Icone::Casa => {
            p.add(Shape::line(vec![pt(-9.0, -1.0), pt(0.0, -9.0), pt(9.0, -1.0)], st));
            p.add(Shape::line(
                vec![pt(-7.0, -2.5), pt(-7.0, 8.5), pt(7.0, 8.5), pt(7.0, -2.5)],
                st,
            ));
            p.add(Shape::line(
                vec![pt(-2.5, 8.5), pt(-2.5, 3.0), pt(2.5, 3.0), pt(2.5, 8.5)],
                st,
            ));
        }
        Icone::Controle => {
            let r = Rect::from_center_size(c, vec2(20.0, 12.0) * s);
            p.add(Shape::closed_line(retangulo_redondo(r, 5.5 * s), st));
            p.line_segment([pt(-6.5, 0.0), pt(-2.5, 0.0)], st);
            p.line_segment([pt(-4.5, -2.0), pt(-4.5, 2.0)], st);
            p.circle_filled(pt(3.5, 1.0), 1.3 * s, cor);
            p.circle_filled(pt(6.5, -1.5), 1.3 * s, cor);
        }
        Icone::Calendario => {
            let r = Rect::from_center_size(pt(0.0, 1.0), vec2(18.0, 17.0) * s);
            p.add(Shape::closed_line(retangulo_redondo(r, 3.0 * s), st));
            p.line_segment([pt(-9.0, -3.0), pt(9.0, -3.0)], st);
            p.line_segment([pt(-4.5, -9.5), pt(-4.5, -5.5)], st);
            p.line_segment([pt(4.5, -9.5), pt(4.5, -5.5)], st);
            p.line_segment([pt(-4.5, 2.5), pt(4.5, 2.5)], st);
            p.line_segment([pt(-4.5, 6.0), pt(1.0, 6.0)], st);
        }
        Icone::Engrenagem => {
            let dentes = 8;
            let mut pts = Vec::new();
            for i in 0..dentes {
                let base = i as f32 / dentes as f32 * TAU;
                let passo = TAU / dentes as f32;
                for (fr, raio) in [(0.0, 7.6), (0.18, 10.0), (0.5, 10.0), (0.68, 7.6)] {
                    let a = base + passo * fr;
                    pts.push(rot(a.cos() * raio, a.sin() * raio));
                }
            }
            p.add(Shape::closed_line(pts, st));
            p.circle_stroke(c, 3.2 * s, st);
        }
        Icone::Atualizar => {
            let mut pts = Vec::new();
            for i in 0..=24 {
                let a = -PI / 2.0 + 0.35 + (TAU - 1.1) * i as f32 / 24.0;
                pts.push(rot(a.cos() * 7.5, a.sin() * 7.5));
            }
            let ponta = *pts.last().unwrap_or(&c);
            p.add(Shape::line(pts, st));
            let a = -PI / 2.0 + 0.35 + (TAU - 1.1);
            let tan = vec2(-a.sin(), a.cos());
            let (sn, cs) = giro.sin_cos();
            let tan = vec2(tan.x * cs - tan.y * sn, tan.x * sn + tan.y * cs);
            let norm = vec2(-tan.y, tan.x);
            p.add(Shape::line(
                vec![
                    ponta - tan * 4.0 * s + norm * 3.5 * s,
                    ponta,
                    ponta - tan * 4.0 * s - norm * 3.5 * s,
                ],
                st,
            ));
        }
        Icone::Voltar => {
            p.add(Shape::line(
                vec![pt(3.0, -6.0), pt(-3.0, 0.0), pt(3.0, 6.0)],
                Stroke::new(2.6 * s, cor),
            ));
        }
        Icone::Trofeu => {
            p.add(Shape::closed_line(
                vec![
                    pt(-6.0, -8.5),
                    pt(6.0, -8.5),
                    pt(6.0, -3.0),
                    pt(4.0, 1.0),
                    pt(0.0, 3.0),
                    pt(-4.0, 1.0),
                    pt(-6.0, -3.0),
                ],
                st,
            ));
            p.add(Shape::line(
                vec![pt(-6.0, -6.5), pt(-9.0, -6.5), pt(-9.0, -3.5), pt(-5.5, -0.5)],
                st,
            ));
            p.add(Shape::line(
                vec![pt(6.0, -6.5), pt(9.0, -6.5), pt(9.0, -3.5), pt(5.5, -0.5)],
                st,
            ));
            p.line_segment([pt(0.0, 3.0), pt(0.0, 7.0)], st);
            p.line_segment([pt(-4.5, 8.5), pt(4.5, 8.5)], st);
        }
        Icone::Relogio => {
            p.circle_stroke(c, 8.5 * s, st);
            p.add(Shape::line(vec![pt(0.0, -4.5), pt(0.0, 0.0), pt(3.5, 2.5)], st));
        }
    }
}

pub fn retangulo_redondo(r: Rect, raio: f32) -> Vec<Pos2> {
    let raio = raio.min(r.width() / 2.0).min(r.height() / 2.0);
    let mut pts = Vec::new();
    let cantos = [
        (pos2(r.right() - raio, r.top() + raio), -PI / 2.0),
        (pos2(r.right() - raio, r.bottom() - raio), 0.0),
        (pos2(r.left() + raio, r.bottom() - raio), PI / 2.0),
        (pos2(r.left() + raio, r.top() + raio), PI),
    ];
    for (c, a0) in cantos {
        for i in 0..=6 {
            let a = a0 + (PI / 2.0) * i as f32 / 6.0;
            pts.push(c + vec2(a.cos(), a.sin()) * raio);
        }
    }
    pts
}

/// Desenha uma textura recortada: círculo (`raio_canto` >= metade do lado) ou quadrado arredondado.
pub fn imagem_recortada(p: &Painter, tex: TextureId, r: Rect, raio_canto: f32, tinta: Color32) {
    let pts: Vec<Pos2> = if raio_canto >= r.width().min(r.height()) / 2.0 - 0.01 {
        (0..48)
            .map(|i| {
                let a = i as f32 / 48.0 * TAU;
                r.center() + vec2(a.cos() * r.width() / 2.0, a.sin() * r.height() / 2.0)
            })
            .collect()
    } else {
        retangulo_redondo(r, raio_canto)
    };
    let mut m = Mesh::with_texture(tex);
    let uv = |q: Pos2| pos2((q.x - r.left()) / r.width(), (q.y - r.top()) / r.height());
    m.vertices.push(Vertex {
        pos: r.center(),
        uv: pos2(0.5, 0.5),
        color: tinta,
    });
    for q in &pts {
        m.vertices.push(Vertex {
            pos: *q,
            uv: uv(*q),
            color: tinta,
        });
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    p.add(Shape::mesh(m));
}

/// O "G" colorido do botão de entrar com Google.
pub fn logo_google(p: &Painter, c: Pos2, raio: f32) {
    let cores = [
        (Color32::from_rgb(0xea, 0x43, 0x35), -2.6f32, -0.85f32),
        (Color32::from_rgb(0xfb, 0xbc, 0x05), -3.6, -2.6),
        (Color32::from_rgb(0x34, 0xa8, 0x53), 0.9, 2.65),
        (Color32::from_rgb(0x42, 0x85, 0xf4), -0.05, 0.9),
    ];
    let esp = raio * 0.38;
    let r = raio - esp / 2.0;
    for (cor, a0, a1) in cores {
        let pts: Vec<Pos2> = (0..=12)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / 12.0;
                c + vec2(a.cos(), a.sin()) * r
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(esp, cor)));
    }
    p.line_segment(
        [c + vec2(0.0, 0.0), c + vec2(raio, 0.0)],
        Stroke::new(esp, Color32::from_rgb(0x42, 0x85, 0xf4)),
    );
}

// ---------- Componentes ----------

/// Avatar quadradinho com a inicial do jogo (cor fixa para cada jogo).
pub fn avatar(ui: &mut Ui, p: &Paleta, nome: &str, pacote: &str, tam: f32, escuro: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(tam), Sense::hover());
    desenhar_avatar(ui.painter(), p, nome, pacote, r, escuro, false);
}

/// Mesma coisa, desenhando num retângulo qualquer. `redondo` = círculo (para pessoas).
pub fn desenhar_avatar(painter: &Painter, p: &Paleta, nome: &str, chave: &str, r: Rect, escuro: f32, redondo: bool) {
    let tam = r.width();
    let mut h: u32 = 2_166_136_261;
    for b in chave.bytes() {
        h = (h ^ b as u32).wrapping_mul(16_777_619);
    }
    let matiz = (h % 360) as f32 / 360.0;
    let fundo_escuro: Color32 = egui::ecolor::Hsva::new(matiz, 0.55, 0.42, 1.0).into();
    let fundo_claro: Color32 = egui::ecolor::Hsva::new(matiz, 0.30, 0.95, 1.0).into();
    let letra_claro: Color32 = egui::ecolor::Hsva::new(matiz, 0.70, 0.45, 1.0).into();
    let fundo = misturar_cor(fundo_claro, fundo_escuro, escuro);
    let letra = misturar_cor(letra_claro, p.text, escuro);
    if redondo {
        painter.circle_filled(r.center(), tam / 2.0, fundo);
    } else {
        painter.rect_filled(r, CornerRadius::same((tam * 0.3) as u8), fundo);
    }
    let inicial = nome.chars().next().unwrap_or('?').to_uppercase().to_string();
    painter.text(
        r.center(),
        Align2::CENTER_CENTER,
        inicial,
        fonte(tam * 0.44, Peso::Negrito),
        letra,
    );
}

/// Barrinha de progresso.
pub fn barra(ui: &mut Ui, p: &Paleta, fracao: f32, cor: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 5.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(r, CornerRadius::same(3), p.chip);
    let mut cheio = r;
    cheio.set_width((r.width() * fracao.clamp(0.0, 1.0)).max(5.0));
    painter.rect_filled(cheio, CornerRadius::same(3), cor);
}

/// Interruptor igual ao `.toggle` do AbyssSub (fica dourado ligado).
pub fn interruptor(ui: &mut Ui, an: &mut Animador, p: &Paleta, chave: &str, ligado: &mut bool) -> Response {
    let (r, mut resp) = ui.allocate_exact_size(vec2(48.0, 28.0), Sense::click());
    if resp.clicked() {
        *ligado = !*ligado;
        resp.mark_changed();
    }
    let t = an.ir(&format!("tg:{chave}"), if *ligado { 1.0 } else { 0.0 }, 0.28);
    let painter = ui.painter();
    painter.rect_filled(r, CornerRadius::same(14), misturar_cor(p.line, p.gold, t));
    let x = egui::lerp(r.left() + 14.0..=r.right() - 14.0, t);
    painter.circle_filled(pos2(x, r.center().y + 1.0), 11.0, alfa(Color32::BLACK, 0.18));
    painter.circle_filled(pos2(x, r.center().y), 11.0, Color32::WHITE);
    resp
}

/// Seletor com uma "bolinha" que desliza (o `.seg` do AbyssSub).
/// `no_topo` = versão translúcida para usar em cima do degradê.
pub fn seletor(
    ui: &mut Ui,
    an: &mut Animador,
    p: &Paleta,
    chave: &str,
    opcoes: &[&str],
    atual: &mut usize,
    no_topo: bool,
) -> bool {
    let n = opcoes.len().max(1);
    let altura = 40.0;
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), altura), Sense::hover());
    let painter = ui.painter().clone();
    let (fundo, bolinha, ativo, inativo) = if no_topo {
        (
            alfa(Color32::WHITE, 0.10),
            alfa(Color32::WHITE, 0.18),
            p.on_hero,
            alfa(p.on_hero, 0.7),
        )
    } else {
        (p.chip, p.seg_on, p.text, p.muted)
    };
    painter.rect_filled(r, CornerRadius::same(14), fundo);
    let dentro = r.shrink(4.0);
    let w = dentro.width() / n as f32;
    let pos = an.ir(&format!("seg:{chave}"), *atual as f32, 0.35);
    let knob = Rect::from_min_size(pos2(dentro.left() + w * pos, dentro.top()), vec2(w, dentro.height()));
    if !no_topo {
        painter.rect_filled(
            knob.translate(vec2(0.0, 1.0)),
            CornerRadius::same(11),
            alfa(Color32::BLACK, 0.12),
        );
    }
    painter.rect_filled(knob, CornerRadius::same(11), bolinha);

    let mut mudou = false;
    for (i, nome) in opcoes.iter().enumerate() {
        let celula = Rect::from_min_size(
            pos2(dentro.left() + w * i as f32, dentro.top()),
            vec2(w, dentro.height()),
        );
        let resp = ui.interact(celula, ui.id().with((chave, i)), Sense::click());
        if resp.clicked() && *atual != i {
            *atual = i;
            mudou = true;
        }
        let perto = (1.0 - (pos - i as f32).abs()).clamp(0.0, 1.0);
        painter.text(
            celula.center(),
            Align2::CENTER_CENTER,
            *nome,
            fonte(13.5, Peso::Semi),
            misturar_cor(inativo, ativo, perto),
        );
    }
    mudou
}

/// Botão quadradinho translúcido do topo (`.iconbtn`). Gira um pouco quando apertado.
pub fn botao_icone(ui: &mut Ui, an: &mut Animador, p: &Paleta, chave: &str, qual: Icone) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
    let apertado = an.ir(
        &format!("ib:{chave}"),
        if resp.is_pointer_button_down_on() { 1.0 } else { 0.0 },
        0.2,
    );
    let painter = ui.painter();
    let rr = Rect::from_center_size(r.center(), r.size() * (1.0 - 0.1 * apertado));
    painter.rect_filled(rr, CornerRadius::same(12), alfa(Color32::WHITE, 0.10 + 0.10 * apertado));
    let giro = match qual {
        Icone::Engrenagem => apertado * 30f32.to_radians(),
        _ => 0.0,
    };
    icone(painter, qual, r.center(), 22.0, p.on_hero, giro);
    resp
}
