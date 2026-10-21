#![allow(dead_code)]

use eframe::egui::{self, Rect, Response, Sense};

use crate::ui::{anim, theme};

pub fn toggle(ui: &mut egui::Ui, rect: Rect, on: &mut bool, s: f32) -> Response {
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id(("toggle", rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let ctx = ui.ctx().clone();
    let t = anim::anim_bool(&ctx, id.with("on"), *on, 0.22, anim::ease_out_back);
    let track = theme::lerp_color(theme::SEGMENT_IDLE, theme::INK, t.clamp(0.0, 1.0));
    let painter = ui.painter();
    painter.rect_filled(rect, theme::corner(rect.height() / 2.0), track);
    let pad = px(3.0);
    let thumb_r = rect.height() / 2.0 - pad;
    let x = egui::lerp(
        (rect.left() + pad + thumb_r)..=(rect.right() - pad - thumb_r),
        t,
    );
    painter.circle_filled(egui::pos2(x, rect.center().y), thumb_r, theme::PANEL);
    if resp.has_focus() {
        theme::focus_ring(
            painter,
            rect.expand(px(3.0)),
            rect.height() / 2.0 + px(3.0),
            theme::INK,
        );
    }
    resp
}
