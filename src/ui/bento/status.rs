#![allow(dead_code)]

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke, StrokeKind};

use crate::ui::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Success,
    Warning,
    Danger,
    Neutral,
}

impl Status {
    pub fn dot(self) -> egui::Color32 {
        match self {
            Status::Success => theme::SUCCESS,
            Status::Warning => theme::WARNING,
            Status::Danger => theme::DANGER,
            Status::Neutral => theme::INK_MUTED,
        }
    }
}

enum Kind {
    Pill,
    Tag,
}

fn badge(ui: &mut egui::Ui, rect: Rect, text: &str, status: Status, s: f32, kind: Kind) {
    let px = |v: f32| theme::px(v, s);
    let (fill, radius_px, outline) = match kind {
        Kind::Pill => (theme::PANEL, rect.height() / 2.0, true),
        Kind::Tag => (theme::PANEL_ALT, px(8.0), false),
    };
    let painter = ui.painter();
    painter.rect_filled(rect, theme::corner(radius_px), fill);
    if outline {
        painter.rect_stroke(
            rect,
            theme::corner(radius_px),
            Stroke::new(1.0, theme::INK),
            StrokeKind::Inside,
        );
    }
    let dot_c = Pos2::new(rect.left() + px(14.0), rect.center().y);
    painter.circle_filled(dot_c, px(4.0), status.dot());
    painter.text(
        Pos2::new(dot_c.x + px(10.0), rect.center().y),
        Align2::LEFT_CENTER,
        text.to_uppercase(),
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK,
    );
}

pub fn pill(ui: &mut egui::Ui, rect: Rect, text: &str, status: Status, s: f32) {
    badge(ui, rect, text, status, s, Kind::Pill);
}

pub fn tag(ui: &mut egui::Ui, rect: Rect, text: &str, status: Status, s: f32) {
    badge(ui, rect, text, status, s, Kind::Tag);
}
