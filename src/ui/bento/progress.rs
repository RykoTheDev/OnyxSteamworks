#![allow(dead_code)]

use eframe::egui::{self, Pos2, Rect, Response, Sense, Stroke, Vec2, epaint::Shape};

use crate::ui::theme;

pub fn progress_bar(ui: &mut egui::Ui, rect: Rect, value: Option<f32>, s: f32) -> Response {
    let _ = s;
    let id = ui.make_persistent_id(("pbar", rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::hover());
    let radius = rect.height() / 2.0;
    ui.painter()
        .rect_filled(rect, theme::corner(radius), theme::TRACK);
    match value {
        Some(v) => {
            let v = ui
                .ctx()
                .animate_value_with_time(id.with("v"), v.clamp(0.0, 1.0), 0.3);
            if v > 0.001 {
                let fill =
                    Rect::from_min_size(rect.min, Vec2::new(rect.width() * v, rect.height()));
                ui.painter()
                    .rect_filled(fill, theme::corner(radius), theme::INK);
            }
        }
        None => {
            ui.ctx().request_repaint();
            let t = ui.input(|i| i.time) as f32;
            let seg_w = rect.width() * 0.3;
            let span = rect.width() - seg_w;
            let x = rect.left() + span * (0.5 + 0.5 * (t * 2.2).sin());
            ui.painter().rect_filled(
                Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(seg_w, rect.height())),
                theme::corner(radius),
                theme::INK,
            );
        }
    }
    resp
}

pub fn progress_ring(ui: &mut egui::Ui, center: Pos2, radius: f32, value: f32, s: f32) -> Response {
    let id = ui.make_persistent_id(("pring", center.x as i32, center.y as i32));
    let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
    let resp = ui.interact(rect, id, Sense::hover());
    let w = theme::px(3.0, s);
    ui.painter()
        .circle_stroke(center, radius, Stroke::new(w, theme::TRACK));
    let v = ui
        .ctx()
        .animate_value_with_time(id.with("v"), value.clamp(0.0, 1.0), 0.3);
    if v > 0.001 {
        let n = 64;
        let start = -std::f32::consts::FRAC_PI_2;
        let pts: Vec<Pos2> = (0..=((n as f32 * v) as usize).max(1))
            .map(|i| {
                center
                    + Vec2::angled(
                        start + std::f32::consts::TAU * v * (i as f32 / (n as f32 * v).max(1.0)),
                    ) * radius
            })
            .collect();
        ui.painter()
            .add(Shape::line(pts, Stroke::new(w, theme::INK)));
    }
    resp
}
