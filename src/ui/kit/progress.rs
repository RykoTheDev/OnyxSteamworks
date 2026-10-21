use eframe::egui::{self, Pos2, Rect, Vec2};

use crate::ui::{anim, theme};

pub fn segment_progress(ui: &mut egui::Ui, rect: Rect, total: usize, filled: usize, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let total = total.max(1);
    let gap = px(6.0);
    let seg_w = (rect.width() - gap * (total as f32 - 1.0)) / total as f32;
    let painter = ui.painter();
    for i in 0..total {
        let x = rect.left() + i as f32 * (seg_w + gap);
        let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(seg_w, rect.height()));
        let on = i < filled;
        painter.rect_filled(
            r,
            theme::corner(rect.height() / 2.0),
            if on { theme::INK } else { theme::SEGMENT_IDLE },
        );
    }
}

pub fn progress_bar(ui: &mut egui::Ui, rect: Rect, frac: f32, s: f32) {
    let id = ui.make_persistent_id(("pbar", rect.left() as i32, rect.top() as i32));
    let t = anim::anim_value(ui.ctx(), id, frac.clamp(0.0, 1.0), 0.2);
    let painter = ui.painter();
    painter.rect_filled(rect, theme::corner(rect.height() / 2.0), theme::TRACK);
    if t > 0.001 {
        let fill = Rect::from_min_size(rect.min, Vec2::new(rect.width() * t, rect.height()));
        painter.rect_filled(fill, theme::corner(rect.height() / 2.0), theme::INK);
    }
    let _ = s;
}
