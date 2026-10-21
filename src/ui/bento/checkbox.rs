#![allow(dead_code)]

use eframe::egui::{self, Pos2, Rect, Response, Sense, Stroke, StrokeKind};

use crate::ui::{anim, theme};

pub fn checkbox(ui: &mut egui::Ui, rect: Rect, checked: &mut bool, s: f32) -> Response {
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id(("checkbox", rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    if resp.clicked() {
        *checked = !*checked;
    }
    let ctx = ui.ctx().clone();
    let t = anim::anim_bool(&ctx, id.with("c"), *checked, 0.2, anim::ease_out_cubic);
    let painter = ui.painter();
    let radius = px(6.0);
    painter.rect_filled(
        rect,
        theme::corner(radius),
        theme::lerp_color(theme::PANEL, theme::INK, t),
    );
    painter.rect_stroke(
        rect,
        theme::corner(radius),
        Stroke::new(1.0, theme::INK),
        StrokeKind::Inside,
    );
    if t > 0.01 {
        draw_check(painter, rect, t, anim::with_opacity(theme::PANEL, t), s);
    }
    if resp.has_focus() {
        theme::focus_ring(painter, rect.expand(px(3.0)), radius + px(3.0), theme::INK);
    }
    resp
}

fn draw_check(painter: &egui::Painter, rect: Rect, t: f32, color: egui::Color32, s: f32) {
    let p = |fx: f32, fy: f32| {
        Pos2::new(
            rect.left() + rect.width() * fx,
            rect.top() + rect.height() * fy,
        )
    };
    let a = p(0.26, 0.52);
    let b = p(0.44, 0.70);
    let c = p(0.74, 0.32);
    let l1 = a.distance(b);
    let l2 = b.distance(c);
    let d = t * (l1 + l2);
    let stroke = Stroke::new(theme::px(2.0, s), color);
    if d <= l1 {
        let e = a + (b - a) * (d / l1);
        painter.line_segment([a, e], stroke);
    } else {
        painter.line_segment([a, b], stroke);
        let e = b + (c - b) * ((d - l1) / l2);
        painter.line_segment([b, e], stroke);
    }
}
