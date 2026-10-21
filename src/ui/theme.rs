#![allow(dead_code)]

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Painter, Rect,
    Stroke, StrokeKind, TextStyle,
};

pub const BG: Color32 = Color32::from_rgb(0x12, 0x12, 0x12);
pub const SURFACE: Color32 = Color32::from_rgb(0x1D, 0x1D, 0x1D);
pub const SURFACE_2: Color32 = Color32::from_rgb(0x24, 0x24, 0x24);
pub const FIELD: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
pub const FIELD_HI: Color32 = Color32::from_rgb(0x34, 0x34, 0x34);
pub const LINE: Color32 = Color32::from_rgb(0x2E, 0x2E, 0x2E);
pub const GHOST: Color32 = Color32::from_rgb(0x45, 0x45, 0x45);

pub const INK: Color32 = Color32::from_rgb(0xE8, 0xE8, 0xE8);
pub const INK_DIM: Color32 = Color32::from_rgb(0xA6, 0xA6, 0xA6);
pub const INK_MUTED: Color32 = Color32::from_rgb(0x8C, 0x8C, 0x8C);
pub const INK_FAINT: Color32 = Color32::from_rgb(0x5E, 0x5E, 0x5E);

pub const PRIMARY: Color32 = Color32::from_rgb(0xED, 0xED, 0xED);
pub const PRIMARY_INK: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
pub const FOCUS: Color32 = Color32::from_rgb(0x5A, 0x5A, 0x5A);

pub const PANEL: Color32 = SURFACE;
pub const PANEL_ALT: Color32 = FIELD;
pub const SEGMENT_IDLE: Color32 = Color32::from_rgb(0x3A, 0x3A, 0x3A);
pub const TRACK: Color32 = Color32::from_rgb(0x2E, 0x2E, 0x2E);

pub const SUCCESS: Color32 = Color32::from_rgb(0x5B, 0xA7, 0x72);
pub const WARNING: Color32 = Color32::from_rgb(0xC7, 0xA2, 0x4C);
pub const DANGER: Color32 = Color32::from_rgb(0xC8, 0x5F, 0x57);

pub const GUTTER: f32 = 24.0;
pub const SPACE_XS: f32 = 8.0;
pub const SPACE_SM: f32 = 12.0;
pub const SPACE_MD: f32 = 16.0;
pub const SPACE_LG: f32 = 24.0;

pub const RADIUS_APP: f32 = 22.0;
pub const RADIUS_PANEL: f32 = 18.0;
pub const RADIUS_PANEL_MIN: f32 = 16.0;
pub const RADIUS_CARD: f32 = 12.0;
pub const RADIUS_FIELD: f32 = 10.0;
pub const RADIUS_POPUP: f32 = 12.0;

pub const REF_W: f32 = 1180.0;
pub const REF_H: f32 = 830.0;
const SCALE_MIN: f32 = 0.78;
const SCALE_MAX: f32 = 1.35;

static UI_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1000);

pub fn set_ui_scale(factor: f32) {
    let factor = factor.clamp(0.8, 1.3);
    UI_SCALE.store(
        (factor * 1000.0) as u32,
        std::sync::atomic::Ordering::Relaxed,
    );
}

pub fn ui_scale() -> f32 {
    UI_SCALE.load(std::sync::atomic::Ordering::Relaxed) as f32 / 1000.0
}

pub fn scale(available: egui::Vec2) -> f32 {
    ((available.x / REF_W).min(available.y / REF_H) * ui_scale()).clamp(SCALE_MIN, SCALE_MAX)
}

pub fn px(value: f32, scale: f32) -> f32 {
    value * scale
}

pub fn corner(radius_px: f32) -> CornerRadius {
    CornerRadius::same(radius_px.round().clamp(0.0, 255.0) as u8)
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn focus_ring(painter: &Painter, rect: Rect, radius_px: f32, color: Color32) {
    painter.rect_stroke(
        rect,
        corner(radius_px),
        Stroke::new(1.5, color),
        StrokeKind::Inside,
    );
}

pub fn hairline(painter: &Painter, rect: Rect, radius_px: f32, color: Color32) {
    painter.rect_stroke(
        rect,
        corner(radius_px),
        Stroke::new(1.0, color),
        StrokeKind::Inside,
    );
}

const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
const INTER_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Inter-Medium.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-Bold.ttf");
const INTER_BLACK_ITALIC: &[u8] = include_bytes!("../../assets/fonts/Inter-BlackItalic.ttf");
const INTER_EXTRABOLD_ITALIC: &[u8] =
    include_bytes!("../../assets/fonts/Inter-ExtraBoldItalic.ttf");
const JETBRAINS_MONO: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
const PHOSPHOR: &[u8] = include_bytes!("../../assets/fonts/Phosphor-Regular.ttf");
const PHOSPHOR_FILL: &[u8] = include_bytes!("../../assets/fonts/Phosphor-Fill.ttf");

pub fn family_medium() -> FontFamily {
    FontFamily::Name("Medium".into())
}

pub fn family_semibold() -> FontFamily {
    FontFamily::Name("SemiBold".into())
}

pub fn family_bold() -> FontFamily {
    FontFamily::Name("Bold".into())
}

pub fn family_display() -> FontFamily {
    FontFamily::Name("Display".into())
}

pub fn family_label() -> FontFamily {
    FontFamily::Name("Label".into())
}

pub fn family_icon() -> FontFamily {
    FontFamily::Name("Phosphor".into())
}

pub fn family_icon_fill() -> FontFamily {
    FontFamily::Name("PhosphorFill".into())
}

pub fn icon_font(size: f32, filled: bool) -> FontId {
    FontId::new(
        size,
        if filled {
            family_icon_fill()
        } else {
            family_icon()
        },
    )
}

pub mod icon {
    pub const HOUSE: char = '\u{e2c2}';
    pub const STACK: char = '\u{e466}';
    pub const HISTORY: char = '\u{e1a0}';
    pub const CLOCK: char = '\u{e19a}';
    pub const BRANCH: char = '\u{e278}';
    pub const FLOW: char = '\u{e6ec}';
    pub const GEAR: char = '\u{e270}';
    pub const SIGN_OUT: char = '\u{e42a}';
    pub const MAGNIFYING_GLASS: char = '\u{e30c}';
    pub const SPARKLE: char = '\u{e6a2}';
    pub const DOWNLOAD: char = '\u{e20c}';
    pub const UPLOAD: char = '\u{e4c0}';
    pub const PLUS: char = '\u{e3d4}';
    pub const COPY: char = '\u{e1ca}';
    pub const TRASH: char = '\u{e4a6}';
    pub const CARET_DOWN: char = '\u{e136}';
    pub const CHECK: char = '\u{e182}';
    pub const FOLDER: char = '\u{e24a}';
    pub const PLAY: char = '\u{e3d0}';
    pub const PACKAGE: char = '\u{e390}';
    pub const ROCKET: char = '\u{e3fe}';
    pub const WARNING: char = '\u{e4e0}';
    pub const X: char = '\u{e4f6}';
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let add = |name: &str, bytes: &'static [u8], defs: &mut FontDefinitions| {
        defs.font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add("Inter", INTER_REGULAR, &mut fonts);
    add("Inter-Medium", INTER_MEDIUM, &mut fonts);
    add("Inter-SemiBold", INTER_SEMIBOLD, &mut fonts);
    add("Inter-Bold", INTER_BOLD, &mut fonts);
    add("Inter-BlackItalic", INTER_BLACK_ITALIC, &mut fonts);
    add("Inter-ExtraBoldItalic", INTER_EXTRABOLD_ITALIC, &mut fonts);
    add("JetBrainsMono", JETBRAINS_MONO, &mut fonts);
    add("Phosphor", PHOSPHOR, &mut fonts);
    add("Phosphor-Fill", PHOSPHOR_FILL, &mut fonts);

    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.insert(0, "Inter".to_owned());
    prop.push("Phosphor".to_owned());

    let mono = fonts.families.entry(FontFamily::Monospace).or_default();
    mono.insert(0, "JetBrainsMono".to_owned());
    mono.push("Phosphor".to_owned());

    let named = [
        (family_medium(), vec!["Inter-Medium", "Inter", "Phosphor"]),
        (
            family_semibold(),
            vec!["Inter-SemiBold", "Inter", "Phosphor"],
        ),
        (family_bold(), vec!["Inter-Bold", "Inter", "Phosphor"]),
        (family_display(), vec!["Inter-BlackItalic", "Inter"]),
        (family_label(), vec!["Inter-ExtraBoldItalic", "Inter"]),
        (family_icon(), vec!["Phosphor"]),
        (family_icon_fill(), vec!["Phosphor-Fill"]),
    ];
    for (family, stack) in named {
        fonts
            .families
            .insert(family, stack.into_iter().map(str::to_owned).collect());
    }
    ctx.set_fonts(fonts);
}

fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    [
        (TextStyle::Heading, FontId::new(16.0, family_semibold())),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        ),
        (TextStyle::Button, FontId::new(14.0, family_medium())),
    ]
    .into()
}

fn build_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(INK);

    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.extreme_bg_color = FIELD;
    v.faint_bg_color = SURFACE_2;
    v.code_bg_color = FIELD;
    v.hyperlink_color = INK;

    v.selection.bg_fill = with_alpha(INK, 40);
    v.selection.stroke = Stroke::new(1.0, INK);

    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.bg_stroke = Stroke::NONE;
        w.fg_stroke = Stroke::new(1.0, INK);
        w.corner_radius = corner(10.0);
        w.expansion = 0.0;
    }
    v.widgets.noninteractive.bg_fill = SURFACE;
    v.widgets.noninteractive.weak_bg_fill = SURFACE;
    v.widgets.inactive.bg_fill = FIELD;
    v.widgets.inactive.weak_bg_fill = FIELD;
    v.widgets.hovered.bg_fill = FIELD_HI;
    v.widgets.hovered.weak_bg_fill = FIELD_HI;
    v.widgets.active.bg_fill = FIELD_HI;
    v.widgets.active.weak_bg_fill = FIELD_HI;
    v
}

pub fn apply_theme(ctx: &egui::Context) {
    install_fonts(ctx);
    let styles = text_styles();
    let visuals = build_visuals();
    ctx.all_styles_mut(|style| {
        style.text_styles = styles.clone();
        style.visuals = visuals.clone();
    });
}
