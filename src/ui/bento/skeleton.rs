#![allow(dead_code)]

use eframe::egui::{self, Rect};

use crate::ui::{anim, theme};

pub fn skeleton(ui: &mut egui::Ui, rect: Rect, radius_px: f32) {
    let fill = if anim::reduce_motion() {
        theme::PANEL_ALT
    } else {
        let t = ui.input(|i| i.time);
        ui.ctx().request_repaint();
        let phase = 0.5 + 0.5 * (t as f32 * 2.0).sin();
        theme::lerp_color(theme::PANEL_ALT, theme::TRACK, phase)
    };
    ui.painter()
        .rect_filled(rect, theme::corner(radius_px), fill);
}
