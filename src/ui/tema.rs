//! Cores e fontes do conjunto Abyss (as mesmas do AbyssSub).

use std::sync::Arc;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, RichText};

#[derive(Clone, Copy)]
pub struct Paleta {
    pub hero_a: Color32,
    pub hero_b: Color32,
    pub on_hero: Color32,
    pub brand: Color32,
    pub brand2: Color32,
    pub gold: Color32,
    pub gold_ink: Color32,
    pub bg: Color32,
    pub card: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub line: Color32,
    pub chip: Color32,
    pub chip_on: Color32,
    pub chip_on_ink: Color32,
    pub field: Color32,
    pub soon_bg: Color32,
    pub soon_ink: Color32,
    pub bar: Color32,
    pub bar_now: Color32,
    pub bar_up: Color32,
    pub overlay: Color32,
    pub toast_bg: Color32,
    pub toast_ink: Color32,
    pub seg_on: Color32,
    pub tab_on: Color32,
    pub primario: Color32,
    pub primario_ink: Color32,
    pub bad: Color32,
}

const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// :root (tema claro) do AbyssSub
pub const CLARO: Paleta = Paleta {
    hero_a: c(0x0e, 0x1c, 0x4a),
    hero_b: c(0x2c, 0x45, 0x97),
    on_hero: c(0xff, 0xff, 0xff),
    brand: c(0x1f, 0x34, 0x78),
    brand2: c(0x6f, 0x8f, 0xe0),
    gold: c(0xe3, 0xad, 0x58),
    gold_ink: c(0x3a, 0x26, 0x08),
    bg: c(0xf3, 0xf5, 0xfb),
    card: c(0xff, 0xff, 0xff),
    text: c(0x0f, 0x17, 0x33),
    muted: c(0x5f, 0x68, 0x88),
    line: c(0xe3, 0xe7, 0xf2),
    chip: c(0xec, 0xef, 0xfa),
    chip_on: c(0x1f, 0x34, 0x78),
    chip_on_ink: c(0xff, 0xff, 0xff),
    field: c(0xf6, 0xf7, 0xfc),
    soon_bg: c(0xfd, 0xec, 0xd6),
    soon_ink: c(0x8f, 0x4f, 0x0a),
    bar: c(0xa9, 0xbb, 0xee),
    bar_now: c(0x1f, 0x34, 0x78),
    bar_up: c(0xe3, 0xa3, 0x4a),
    overlay: Color32::from_rgba_premultiplied(4, 7, 20, 128),
    toast_bg: c(0x0f, 0x17, 0x33),
    toast_ink: c(0xff, 0xff, 0xff),
    seg_on: c(0xff, 0xff, 0xff),
    tab_on: c(0xec, 0xef, 0xfa),
    primario: c(0x2c, 0x45, 0x97),
    primario_ink: c(0xff, 0xff, 0xff),
    bad: c(0xd6, 0x45, 0x45),
};

/// :root[data-theme="dark"] do AbyssSub
pub const ESCURO: Paleta = Paleta {
    hero_a: c(0x0a, 0x15, 0x38),
    hero_b: c(0x1b, 0x2f, 0x6b),
    on_hero: c(0xee, 0xf1, 0xfb),
    brand: c(0x8e, 0xa7, 0xf0),
    brand2: c(0x6f, 0x8f, 0xe0),
    gold: c(0xe8, 0xb7, 0x68),
    gold_ink: c(0x2b, 0x1c, 0x05),
    bg: c(0x07, 0x0d, 0x22),
    card: c(0x0f, 0x18, 0x38),
    text: c(0xe8, 0xec, 0xf8),
    muted: c(0x95, 0xa0, 0xc4),
    line: c(0x1e, 0x2a, 0x55),
    chip: c(0x17, 0x22, 0x4a),
    chip_on: c(0xe8, 0xb7, 0x68),
    chip_on_ink: c(0x2b, 0x1c, 0x05),
    field: c(0x0b, 0x14, 0x30),
    soon_bg: c(0x3d, 0x2b, 0x10),
    soon_ink: c(0xff, 0xcf, 0x8a),
    bar: c(0x2e, 0x42, 0x87),
    bar_now: c(0x8e, 0xa7, 0xf0),
    bar_up: c(0xe8, 0xb7, 0x68),
    overlay: Color32::from_rgba_premultiplied(0, 0, 0, 153),
    toast_bg: c(0xe8, 0xec, 0xf8),
    toast_ink: c(0x0f, 0x17, 0x33),
    seg_on: c(0x22, 0x30, 0x6a),
    tab_on: c(0x22, 0x30, 0x6a),
    primario: c(0xe8, 0xb7, 0x68),
    primario_ink: c(0x2b, 0x1c, 0x05),
    bad: c(0xd6, 0x45, 0x45),
};

pub fn misturar_cor(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

/// Cor com transparência (0..1), como rgba(...) do CSS.
pub fn alfa(cor: Color32, a: f32) -> Color32 {
    cor.gamma_multiply(a.clamp(0.0, 1.0))
}

impl Paleta {
    /// t = 0 -> claro, t = 1 -> escuro (usado na transição de tema de 0,4 s).
    pub fn misturar(t: f32) -> Paleta {
        let (a, b) = (&CLARO, &ESCURO);
        let m = |x: Color32, y: Color32| misturar_cor(x, y, t);
        Paleta {
            hero_a: m(a.hero_a, b.hero_a),
            hero_b: m(a.hero_b, b.hero_b),
            on_hero: m(a.on_hero, b.on_hero),
            brand: m(a.brand, b.brand),
            brand2: m(a.brand2, b.brand2),
            gold: m(a.gold, b.gold),
            gold_ink: m(a.gold_ink, b.gold_ink),
            bg: m(a.bg, b.bg),
            card: m(a.card, b.card),
            text: m(a.text, b.text),
            muted: m(a.muted, b.muted),
            line: m(a.line, b.line),
            chip: m(a.chip, b.chip),
            chip_on: m(a.chip_on, b.chip_on),
            chip_on_ink: m(a.chip_on_ink, b.chip_on_ink),
            field: m(a.field, b.field),
            soon_bg: m(a.soon_bg, b.soon_bg),
            soon_ink: m(a.soon_ink, b.soon_ink),
            bar: m(a.bar, b.bar),
            bar_now: m(a.bar_now, b.bar_now),
            bar_up: m(a.bar_up, b.bar_up),
            overlay: m(a.overlay, b.overlay),
            toast_bg: m(a.toast_bg, b.toast_bg),
            toast_ink: m(a.toast_ink, b.toast_ink),
            seg_on: m(a.seg_on, b.seg_on),
            tab_on: m(a.tab_on, b.tab_on),
            primario: m(a.primario, b.primario),
            primario_ink: m(a.primario_ink, b.primario_ink),
            bad: m(a.bad, b.bad),
        }
    }
}

// ---------- Fontes (Inter, a mesma cara do AbyssSub) ----------

#[derive(Clone, Copy)]
pub enum Peso {
    Normal,
    Semi,
    Negrito,
    Pesado,
}

fn familia(p: Peso) -> FontFamily {
    match p {
        Peso::Normal => FontFamily::Proportional,
        Peso::Semi => FontFamily::Name("semi".into()),
        Peso::Negrito => FontFamily::Name("negrito".into()),
        Peso::Pesado => FontFamily::Name("pesado".into()),
    }
}

pub fn fonte(tamanho: f32, peso: Peso) -> FontId {
    FontId::new(tamanho, familia(peso))
}

pub fn txt(texto: impl Into<String>, tamanho: f32, peso: Peso, cor: Color32) -> RichText {
    RichText::new(texto).font(fonte(tamanho, peso)).color(cor)
}

pub fn instalar_fontes(ctx: &egui::Context) {
    let mut f = FontDefinitions::default();
    let mut add = |nome: &str, bytes: &'static [u8]| {
        f.font_data
            .insert(nome.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add("inter", include_bytes!("../../assets/fontes/Inter-Regular.ttf"));
    add("inter-semi", include_bytes!("../../assets/fontes/Inter-SemiBold.ttf"));
    add("inter-negrito", include_bytes!("../../assets/fontes/Inter-Bold.ttf"));
    add(
        "inter-pesado",
        include_bytes!("../../assets/fontes/Inter-ExtraBold.ttf"),
    );

    // As fontes padrão do egui ficam como reserva (símbolos que a Inter não tem).
    let reserva: Vec<String> = f.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let com_reserva = |principal: &str| {
        let mut v = vec![principal.to_owned()];
        v.extend(reserva.iter().cloned());
        v
    };
    f.families.insert(FontFamily::Proportional, com_reserva("inter"));
    f.families
        .insert(FontFamily::Name("semi".into()), com_reserva("inter-semi"));
    f.families
        .insert(FontFamily::Name("negrito".into()), com_reserva("inter-negrito"));
    f.families
        .insert(FontFamily::Name("pesado".into()), com_reserva("inter-pesado"));
    ctx.set_fonts(f);
}

pub fn aplicar_estilo(ctx: &egui::Context, p: &Paleta, escuro: bool) {
    use egui::{vec2, TextStyle};
    let mut v = if escuro {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.extreme_bg_color = p.field;
    v.override_text_color = Some(p.text);
    v.widgets.noninteractive.bg_stroke.color = p.line;
    v.widgets.inactive.weak_bg_fill = p.chip;
    v.widgets.inactive.bg_fill = p.chip;
    v.widgets.hovered.weak_bg_fill = p.chip;
    v.widgets.active.weak_bg_fill = p.chip;
    v.selection.bg_fill = alfa(p.brand2, 0.35);
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.text_styles = [
            (TextStyle::Heading, fonte(20.0, Peso::Negrito)),
            (TextStyle::Body, fonte(15.0, Peso::Normal)),
            (TextStyle::Button, fonte(15.0, Peso::Semi)),
            (TextStyle::Small, fonte(12.5, Peso::Normal)),
            (TextStyle::Monospace, FontId::monospace(14.0)),
        ]
        .into();
        s.spacing.item_spacing = vec2(10.0, 10.0);
        s.spacing.button_padding = vec2(14.0, 10.0);
        // Barra de rolagem mais grossa, fácil de pegar com o dedo.
        s.spacing.scroll = egui::style::ScrollStyle {
            bar_width: 16.0,
            floating_width: 7.0,
            handle_min_length: 56.0,
            dormant_handle_opacity: 0.45,
            active_handle_opacity: 0.8,
            interact_handle_opacity: 1.0,
            ..egui::style::ScrollStyle::floating()
        };
        // No celular, segurar um texto não deve "roubar" o toque do cartão.
        s.interaction.selectable_labels = false;
        s.animation_time = 0.25;
    });
}
