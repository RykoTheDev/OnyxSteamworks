#![allow(dead_code)]

use eframe::egui::{self, Align2, Pos2, Rect, Response, Sense, Vec2};

use crate::ui::{anim, theme};

pub fn notch_button(ui: &mut egui::Ui, center: Pos2, s: f32) -> Response {
    let px = |v: f32| theme::px(v, s);
    let outer_d = px(104.0);
    let inner_d = px(76.0);

    let hit = Rect::from_center_size(center, Vec2::splat(outer_d));
    let id = ui.make_persistent_id(("notch_button", center.x as i32, center.y as i32));
    let resp = ui.interact(hit, id, Sense::click());

    let ctx = ui.ctx();
    let hover = anim::anim_bool(ctx, id, resp.hovered(), 0.14, anim::ease_out_cubic);
    let press = anim::anim_bool(
        ctx,
        id.with("press"),
        resp.is_pointer_button_down_on(),
        0.09,
        anim::ease_out_cubic,
    );
    let scale = 1.0 + 0.03 * hover - 0.06 * press;

    let painter = ui.painter();
    painter.circle_filled(center, outer_d / 2.0, theme::BG);
    painter.circle_filled(center, inner_d / 2.0 * scale, theme::PANEL);
    painter.text(
        center,
        Align2::CENTER_CENTER,
        theme::icon::DOWNLOAD.to_string(),
        theme::icon_font(px(24.0), false),
        theme::INK,
    );
    resp
}
