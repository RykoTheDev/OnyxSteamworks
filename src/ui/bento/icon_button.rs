#![allow(dead_code)]

use eframe::egui::{self, Align2, Pos2, Rect, Response, Sense, Vec2};

use crate::ui::{anim, theme};

pub fn icon_button(ui: &mut egui::Ui, center: Pos2, glyph: char, s: f32) -> Response {
    let px = |v: f32| theme::px(v, s);
    let r = px(22.0);
    let rect = Rect::from_center_size(center, Vec2::splat(r * 2.0));
    let id = ui.make_persistent_id(("icon_button", center.x as i32, center.y as i32));
    let resp = ui.interact(rect, id, Sense::click());
    let ctx = ui.ctx().clone();
    let hover = anim::anim_bool(&ctx, id, resp.hovered(), 0.14, anim::ease_out_cubic);
    let press = anim::anim_bool(
        &ctx,
        id.with("p"),
        resp.is_pointer_button_down_on(),
        0.09,
        anim::ease_out_cubic,
    );
    let painter = ui.painter();
    painter.circle_filled(center, r, anim::with_opacity(theme::PANEL_ALT, hover));
    let size = px(20.0) * (1.0 + 0.03 * hover - 0.06 * press);
    painter.text(
        center,
        Align2::CENTER_CENTER,
        glyph.to_string(),
        theme::icon_font(size, false),
        theme::INK,
    );
    if resp.has_focus() {
        theme::focus_ring(painter, rect, r, theme::INK);
    }
    resp
}
