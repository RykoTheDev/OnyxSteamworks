use eframe::egui::{self, Align2, FontId, Rect, Response, Sense, Vec2};

use crate::ui::{anim, theme};

pub fn chip_size(ui: &egui::Ui, label: &str, s: f32) -> Vec2 {
    let px = |v: f32| theme::px(v, s);
    let font = FontId::new(px(12.0), theme::family_medium());
    let w = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, theme::INK)
        .size()
        .x;
    Vec2::new(w + px(24.0), px(30.0))
}

pub fn chip(ui: &mut egui::Ui, rect: Rect, label: &str, selected: bool, s: f32) -> Response {
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id(("chip", label, rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    let ctx = ui.ctx().clone();
    let sel = anim::anim_bool(&ctx, id.with("s"), selected, 0.16, anim::ease_out_cubic);
    let hover = anim::anim_bool(
        &ctx,
        id.with("h"),
        resp.hovered(),
        0.12,
        anim::ease_out_cubic,
    );
    let fill = theme::lerp_color(
        theme::lerp_color(theme::SURFACE_2, theme::FIELD, hover),
        theme::FIELD_HI,
        sel,
    );
    let painter = ui.painter();
    painter.rect_filled(rect, theme::corner(px(8.0)), fill);
    let text = theme::lerp_color(theme::INK_MUTED, theme::INK, sel.max(hover * 0.6));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::new(px(12.0), theme::family_medium()),
        text,
    );
    resp
}
