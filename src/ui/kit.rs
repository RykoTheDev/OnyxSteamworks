#![allow(dead_code)]
#![allow(unused_imports)]

pub mod button;
pub mod input;
pub mod overlay;
pub mod rail;
pub mod select;
pub mod toggles;

pub use button::{Button, ButtonKind, icon_button};
pub use input::Input;
pub use overlay::{ToastLevel, Toasts, modal, tooltip};
pub use rail::{RailAction, rail};
pub use select::{dropdown, tabs};
pub use toggles::{checkbox, toggle};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::ui::theme;

pub fn surface(painter: &egui::Painter, rect: Rect, fill: egui::Color32, radius_px: f32) {
    painter.rect_filled(rect, theme::corner(radius_px), fill);
}

pub fn panel(painter: &egui::Painter, rect: Rect, fill: egui::Color32, radius_px: f32) {
    surface(painter, rect, fill, radius_px);
}

pub fn bordered(
    painter: &egui::Painter,
    rect: Rect,
    fill: egui::Color32,
    border: egui::Color32,
    radius_px: f32,
) {
    painter.rect_filled(rect, theme::corner(radius_px), fill);
    painter.rect_stroke(
        rect,
        theme::corner(radius_px),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
}

pub fn separator(painter: &egui::Painter, a: Pos2, b: Pos2) {
    painter.line_segment([a, b], Stroke::new(1.0, theme::LINE));
}

pub fn hero(painter: &egui::Painter, clip: Rect, text: &str, s: f32) {
    let size = theme::px(104.0, s);
    let pos = Pos2::new(clip.left() - theme::px(4.0, s), clip.top() - theme::px(18.0, s));
    let p = painter.with_clip_rect(clip);
    p.text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::new(size, theme::family_display()),
        theme::GHOST,
    );
}

pub fn label(painter: &egui::Painter, pos: Pos2, text: &str, s: f32) {
    painter.text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::new(theme::px(13.0, s), theme::family_label()),
        theme::INK_MUTED,
    );
}

pub fn label_optional(painter: &egui::Painter, pos: Pos2, text: &str, suffix: &str, s: f32) {
    let font = FontId::new(theme::px(13.0, s), theme::family_label());
    let w = painter
        .layout_no_wrap(text.to_owned(), font.clone(), theme::INK_MUTED)
        .size()
        .x;
    painter.text(pos, Align2::LEFT_TOP, text, font, theme::INK_MUTED);
    painter.text(
        Pos2::new(pos.x + w + theme::px(8.0, s), pos.y),
        Align2::LEFT_TOP,
        suffix,
        FontId::new(theme::px(11.0, s), theme::family_label()),
        theme::INK_FAINT,
    );
}

pub fn heading(painter: &egui::Painter, pos: Pos2, text: &str, s: f32) {
    painter.text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::new(theme::px(15.0, s), theme::family_semibold()),
        theme::INK,
    );
}

pub fn caption(painter: &egui::Painter, pos: Pos2, text: &str, s: f32) {
    painter.text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::new(theme::px(12.0, s), theme::family_medium()),
        theme::INK_MUTED,
    );
}

pub fn spinner(painter: &egui::Painter, center: Pos2, r: f32, color: egui::Color32, ctx: &egui::Context) {
    ctx.request_repaint();
    let t = ctx.input(|i| i.time) as f32;
    let start = t * 5.0;
    let sweep = 4.6;
    let n = 24;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| center + Vec2::angled(start + sweep * (i as f32 / n as f32)) * r)
        .collect();
    painter.add(egui::epaint::Shape::line(pts, Stroke::new(2.0, color)));
}

pub fn image_or_placeholder(
    ui: &mut egui::Ui,
    rect: Rect,
    uri: Option<&str>,
    letter: char,
    radius_px: f32,
    s: f32,
) {
    match uri {
        Some(u) if !u.is_empty() => {
            egui::Image::from_uri(u.to_owned())
                .maintain_aspect_ratio(false)
                .fit_to_exact_size(rect.size())
                .corner_radius(theme::corner(radius_px))
                .paint_at(ui, rect);
        }
        _ => {
            let painter = ui.painter();
            bordered(painter, rect, theme::FIELD, theme::LINE, radius_px);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                letter.to_uppercase().to_string(),
                FontId::new(theme::px(20.0, s), theme::family_semibold()),
                theme::INK_MUTED,
            );
        }
    }
}

pub fn empty_state(ui: &mut egui::Ui, rect: Rect, glyph: char, title: &str, body: &str, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let painter = ui.painter();
    let cx = rect.center().x;
    let cy = rect.center().y;
    painter.text(
        Pos2::new(cx, cy - px(34.0)),
        Align2::CENTER_CENTER,
        glyph.to_string(),
        theme::icon_font(px(34.0), false),
        theme::INK_MUTED,
    );
    painter.text(
        Pos2::new(cx, cy),
        Align2::CENTER_CENTER,
        title,
        FontId::new(px(15.0), theme::family_semibold()),
        theme::INK_DIM,
    );
    painter.text(
        Pos2::new(cx, cy + px(22.0)),
        Align2::CENTER_CENTER,
        body,
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_MUTED,
    );
}
