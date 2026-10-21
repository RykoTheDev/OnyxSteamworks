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

pub fn progress_ring(
    ui: &mut egui::Ui,
    center: Pos2,
    radius: f32,
    frac: f32,
    color: egui::Color32,
    s: f32,
) {
    let id = ui.make_persistent_id(("ring", center.x as i32, center.y as i32));
    let t = anim::anim_value(ui.ctx(), id, frac.clamp(0.0, 1.0), 0.25);
    let stroke_w = theme::px(5.0, s);
    let painter = ui.painter();
    painter.circle_stroke(center, radius, egui::Stroke::new(stroke_w, theme::TRACK));
    if t > 0.001 {
        let steps = (96.0 * t).ceil().max(2.0) as usize;
        let start = -std::f32::consts::FRAC_PI_2;
        let sweep = std::f32::consts::TAU * t;
        let points: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let a = start + sweep * (i as f32 / steps as f32);
                Pos2::new(center.x + radius * a.cos(), center.y + radius * a.sin())
            })
            .collect();
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(stroke_w, color),
        ));
    }
}

pub fn line_chart(ui: &mut egui::Ui, rect: Rect, samples: &[f32], color: egui::Color32, s: f32) {
    if samples.len() < 2 {
        return;
    }
    let max = samples.iter().copied().fold(f32::MIN, f32::max).max(1e-6);
    let n = samples.len();
    let points: Vec<Pos2> = samples
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = rect.left() + rect.width() * (i as f32 / (n - 1) as f32);
            let y = rect.bottom() - rect.height() * (v / max).clamp(0.0, 1.0);
            Pos2::new(x, y)
        })
        .collect();
    ui.painter().add(egui::Shape::line(
        points,
        egui::Stroke::new(theme::px(2.0, s), color),
    ));
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
