#![allow(dead_code)]

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Vec2};

use crate::ui::theme;

pub fn tabs(
    ui: &mut egui::Ui,
    rect: Rect,
    labels: &[&str],
    active: &mut usize,
    s: f32,
) -> egui::Response {
    let px = |v: f32| theme::px(v, s);
    let n = labels.len().max(1);
    let seg_w = rect.width() / n as f32;
    let id = ui.make_persistent_id(("tabs", rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::hover());

    ui.painter()
        .rect_filled(rect, theme::corner(rect.height() / 2.0), theme::PANEL);

    let target = rect.left() + seg_w * (*active as f32);
    let x = ui
        .ctx()
        .animate_value_with_time(id.with("ind"), target, 0.2);
    let ind = Rect::from_min_size(
        Pos2::new(x + px(3.0), rect.top() + px(3.0)),
        Vec2::new(seg_w - px(6.0), rect.height() - px(6.0)),
    );
    ui.painter()
        .rect_filled(ind, theme::corner(ind.height() / 2.0), theme::INK);

    for (i, label) in labels.iter().enumerate() {
        let seg = Rect::from_min_size(
            Pos2::new(rect.left() + seg_w * i as f32, rect.top()),
            Vec2::new(seg_w, rect.height()),
        );
        if ui.interact(seg, id.with(i), Sense::click()).clicked() {
            *active = i;
        }
        let on = i == *active;
        ui.painter().text(
            seg.center(),
            Align2::CENTER_CENTER,
            *label,
            FontId::new(px(12.0), theme::family_medium()),
            if on { theme::PANEL } else { theme::INK },
        );
    }
    resp
}
