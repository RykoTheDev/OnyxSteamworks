#![allow(dead_code)]

use eframe::egui::{self, Align2, Key, Pos2, Rect, TextEdit};

use super::panel;
use crate::ui::theme;

pub fn prompt_bar(ui: &mut egui::Ui, rect: Rect, text: &mut String, s: f32) -> bool {
    let px = |v: f32| theme::px(v, s);
    let painter = ui.painter().clone();
    panel::panel(&painter, rect, theme::PANEL, rect.height() / 2.0);

    let circle_r = px(20.0);
    let circle_c = Pos2::new(rect.left() + px(4.0) + circle_r, rect.center().y);
    painter.circle_filled(circle_c, circle_r, theme::INK);
    painter.text(
        circle_c,
        Align2::CENTER_CENTER,
        theme::icon::SPARKLE.to_string(),
        theme::icon_font(px(18.0), false),
        theme::PANEL,
    );

    let text_rect = Rect::from_min_max(
        Pos2::new(circle_c.x + circle_r + px(8.0), rect.top()),
        Pos2::new(rect.right() - px(16.0), rect.bottom()),
    );
    let resp = ui.put(
        text_rect,
        TextEdit::singleline(text)
            .frame(egui::Frame::NONE)
            .hint_text("Describe a build")
            .desired_width(f32::INFINITY),
    );
    resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
}
