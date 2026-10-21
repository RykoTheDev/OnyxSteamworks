#![allow(dead_code)]

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Painter, Rect,
    Stroke, StrokeKind, TextStyle,
};

// --- Tokens ---------------------------------------------------------------

pub const BG: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
pub const PANEL: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const PANEL_ALT: Color32 = Color32::from_rgb(0xF4, 0xF4, 0xF4);
pub const INK: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0A);
pub const INK_MUTED: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x8A);
pub const SEGMENT_IDLE: Color32 = Color32::from_rgb(0xCF, 0xCF, 0xCF);
pub const TRACK: Color32 = Color32::from_rgb(0xE6, 0xE6, 0xE6);

// Status colors: only ever tiny dots or text, never fills or large areas.
pub const SUCCESS: Color32 = Color32::from_rgb(0x34, 0xD3, 0x99);
pub const WARNING: Color32 = Color32::from_rgb(0xFB, 0xBF, 0x24);
pub const DANGER: Color32 = Color32::from_rgb(0xF8, 0x71, 0x71);

// --- Spacing and radii (reference units; pass through px()) ----------------

pub const GUTTER: f32 = 24.0;
pub const SPACE_XS: f32 = 8.0;
pub const SPACE_SM: f32 = 12.0;
pub const SPACE_MD: f32 = 16.0;
pub const SPACE_LG: f32 = 24.0;

pub const RADIUS_PANEL: f32 = 56.0;
pub const RADIUS_PANEL_MIN: f32 = 48.0;
pub const RADIUS_CARD: f32 = 40.0;
pub const RADIUS_POPUP: f32 = 24.0;

// --- Reference canvas and scaling -----------------------------------------

pub const REF_W: f32 = 1180.0;
pub const REF_H: f32 = 830.0;
const SCALE_MIN: f32 = 0.75;
const SCALE_MAX: f32 = 1.4;

/// Scale factor for the current window size: `min(w/1180, h/830)` clamped to
/// the range 0.75 to 1.4.
pub fn scale(available: egui::Vec2) -> f32 {
    (available.x / REF_W)
        .min(available.y / REF_H)
        .clamp(SCALE_MIN, SCALE_MAX)
}

/// Convert a reference-unit value to pixels for the given `scale`.
pub fn px(value: f32, scale: f32) -> f32 {
    value * scale
}

/// A `CornerRadius` from a (scaled) pixel radius.
pub fn corner(radius_px: f32) -> CornerRadius {
    CornerRadius::same(radius_px.round().clamp(0.0, 255.0) as u8)
}

// --- Color helpers --------------------------------------------------------

/// Linearly interpolate between two solid colors (for cross-fades).
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

/// The same color at a different alpha (0 to 255). Useful for faint outlines.
pub fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

/// Draw a 2px focus ring around `rect`. `color` is `INK` on white panels and
/// white (`PANEL`) on the black canvas.
pub fn focus_ring(painter: &Painter, rect: Rect, radius_px: f32, color: Color32) {
    painter.rect_stroke(
        rect,
        corner(radius_px),
        Stroke::new(2.0, color),
        StrokeKind::Inside,
    );
}

// --- Fonts ----------------------------------------------------------------

const MANROPE_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Manrope-Regular.ttf");
const MANROPE_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Manrope-Medium.ttf");
const MANROPE_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Manrope-SemiBold.ttf");
const JETBRAINS_MONO: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
const PHOSPHOR: &[u8] = include_bytes!("../../assets/fonts/Phosphor-Regular.ttf");
const PHOSPHOR_FILL: &[u8] = include_bytes!("../../assets/fonts/Phosphor-Fill.ttf");

/// Named family for Manrope Medium (labels, buttons).
pub fn family_medium() -> FontFamily {
    FontFamily::Name("Medium".into())
}

/// Named family for Manrope SemiBold (headings, titles).
pub fn family_semibold() -> FontFamily {
    FontFamily::Name("SemiBold".into())
}

pub fn family_icon() -> FontFamily {
    FontFamily::Name("Phosphor".into())
}

/// Named family for the filled phosphor weight (active icons).
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
    pub const HEART: char = '\u{e2a8}';
    pub const IMAGE: char = '\u{e2ca}';
    pub const CROP: char = '\u{e1d4}';
    pub const GEAR: char = '\u{e270}';
    pub const SIGN_OUT: char = '\u{e42a}';
    pub const MAGNIFYING_GLASS: char = '\u{e30c}';
    pub const SPARKLE: char = '\u{e6a2}';
    pub const DOWNLOAD: char = '\u{e20a}';
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let add = |name: &str, bytes: &'static [u8], defs: &mut FontDefinitions| {
        defs.font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add("Manrope", MANROPE_REGULAR, &mut fonts);
    add("Manrope-Medium", MANROPE_MEDIUM, &mut fonts);
    add("Manrope-SemiBold", MANROPE_SEMIBOLD, &mut fonts);
    add("JetBrainsMono", JETBRAINS_MONO, &mut fonts);
    add("Phosphor", PHOSPHOR, &mut fonts);
    add("Phosphor-Fill", PHOSPHOR_FILL, &mut fonts);

    // Manrope leads the proportional stack; phosphor trails as an icon fallback.
    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.insert(0, "Manrope".to_owned());
    prop.push("Phosphor".to_owned());

    let mono = fonts.families.entry(FontFamily::Monospace).or_default();
    mono.insert(0, "JetBrainsMono".to_owned());
    mono.push("Phosphor".to_owned());

    fonts.families.insert(
        family_medium(),
        vec!["Manrope-Medium".into(), "Manrope".into(), "Phosphor".into()],
    );
    fonts.families.insert(
        family_semibold(),
        vec![
            "Manrope-SemiBold".into(),
            "Manrope".into(),
            "Phosphor".into(),
        ],
    );
    fonts
        .families
        .insert(FontFamily::Name("Phosphor".into()), vec!["Phosphor".into()]);
    fonts
        .families
        .insert(family_icon_fill(), vec!["Phosphor-Fill".into()]);

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

// --- Visuals --------------------------------------------------------------

fn build_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    v.dark_mode = false;
    v.override_text_color = None;

    // The canvas and any window chrome are pure black; panels we paint white.
    v.panel_fill = BG;
    v.window_fill = BG;
    v.window_stroke = Stroke::NONE;
    v.extreme_bg_color = PANEL_ALT;
    v.faint_bg_color = PANEL_ALT;
    v.code_bg_color = PANEL_ALT;
    v.hyperlink_color = INK;

    // Text selection: black highlight, white text.
    v.selection.bg_fill = INK;
    v.selection.stroke = Stroke::new(1.0, PANEL);

    // Base widget states: white/alt fills, ink text, no strokes. Chip outlines
    // and focus rings are painted explicitly by the bento kit, not here.
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.bg_stroke = Stroke::NONE;
        w.fg_stroke = Stroke::new(1.0, INK);
        w.corner_radius = corner(12.0);
        w.expansion = 0.0;
    }
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.weak_bg_fill = PANEL;
    v.widgets.inactive.bg_fill = PANEL_ALT;
    v.widgets.inactive.weak_bg_fill = PANEL_ALT;
    v.widgets.hovered.bg_fill = PANEL_ALT;
    v.widgets.hovered.weak_bg_fill = PANEL_ALT;
    v.widgets.active.bg_fill = PANEL_ALT;
    v.widgets.active.weak_bg_fill = PANEL_ALT;

    v
}

/// Embed the fonts, define the text styles, and apply the flat monochrome
/// visuals. Call once at startup.
pub fn apply_theme(ctx: &egui::Context) {
    install_fonts(ctx);
    let styles = text_styles();
    let visuals = build_visuals();
    ctx.all_styles_mut(|style| {
        style.text_styles = styles.clone();
        style.visuals = visuals.clone();
    });
}
