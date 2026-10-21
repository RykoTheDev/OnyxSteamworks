#![allow(dead_code)]

use eframe::egui::{self, Pos2, Rect, Vec2};

use crate::ui::{anim, theme};

pub fn segment_progress(ui: &mut egui::Ui, top_left: Pos2, count: usize, active: usize, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let w = px(28.0);
    let h = px(3.0);
    let gap = px(6.0);
    let ctx = ui.ctx().clone();
    for i in 0..count {
        let r = Rect::from_min_size(
            Pos2::new(top_left.x + i as f32 * (w + gap), top_left.y),
            Vec2::new(w, h),
        );
        let id = ui.make_persistent_id(("seg", i, top_left.x as i32, top_left.y as i32));
        let t = anim::anim_bool(&ctx, id, i == active, 0.2, anim::ease_out_cubic);
        ui.painter().rect_filled(
            r,
            theme::corner(h / 2.0),
            theme::lerp_color(theme::SEGMENT_IDLE, theme::INK, t),
        );
    }
}
