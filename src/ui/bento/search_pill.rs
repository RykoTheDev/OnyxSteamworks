#![allow(dead_code)]

use eframe::egui::{self, Align2, Pos2, Rect, Response, TextEdit};

use super::panel;
use crate::ui::theme;

pub fn search_pill(ui: &mut egui::Ui, rect: Rect, text: &mut String, s: f32) -> Response {
    let px = |v: f32| theme::px(v, s);
    let painter = ui.painter().clone();
    panel::panel(&painter, rect, theme::PANEL, rect.height() / 2.0);

    let icon_center = Pos2::new(rect.left() + px(24.0) + px(12.0), rect.center().y);
    painter.text(
        icon_center,
        Align2::CENTER_CENTER,
        theme::icon::MAGNIFYING_GLASS.to_string(),
        theme::icon_font(px(24.0), false),
        theme::INK_MUTED,
    );

    let text_rect = Rect::from_min_max(
        Pos2::new(icon_center.x + px(24.0), rect.top()),
        Pos2::new(rect.right() - px(20.0), rect.bottom()),
    );
    ui.put(
        text_rect,
        TextEdit::singleline(text)
            .frame(egui::Frame::NONE)
            .hint_text("Search")
            .desired_width(f32::INFINITY),
    )
}
