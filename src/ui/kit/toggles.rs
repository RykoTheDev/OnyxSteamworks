use eframe::egui::{self, Align2, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Vec2};

use crate::ui::{anim, theme};

pub fn checkbox(
    ui: &mut egui::Ui,
    rect: Rect,
    checked: &mut bool,
    label: &str,
    s: f32,
) -> Response {
    let px = |v: f32| theme::px(v, s);
    let box_size = px(18.0);
    let box_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.center().y - box_size / 2.0),
        Vec2::splat(box_size),
    );
    let id = ui.make_persistent_id(("checkbox", rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    if resp.clicked() {
        *checked = !*checked;
    }
    let ctx = ui.ctx().clone();
    let t = anim::anim_bool(&ctx, id.with("c"), *checked, 0.18, anim::ease_out_cubic);
    let painter = ui.painter();
    let radius = px(5.0);
    painter.rect_filled(
        box_rect,
        theme::corner(radius),
        theme::lerp_color(theme::FIELD, theme::INK, t),
    );
    painter.rect_stroke(
        box_rect,
        theme::corner(radius),
        Stroke::new(1.0, theme::lerp_color(theme::LINE, theme::INK, t)),
        StrokeKind::Inside,
    );
    if t > 0.01 {
        draw_check(painter, box_rect, t, theme::PRIMARY_INK, s);
    }
    if !label.is_empty() {
        painter.text(
            Pos2::new(box_rect.right() + px(10.0), rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::new(px(13.0), theme::family_medium()),
            theme::INK_DIM,
        );
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

pub fn toggle(ui: &mut egui::Ui, rect: Rect, on: &mut bool, s: f32) -> Response {
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id(("toggle", rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let ctx = ui.ctx().clone();
    let t = anim::anim_bool(&ctx, id.with("on"), *on, 0.2, anim::ease_out_back);
    let track = theme::lerp_color(theme::FIELD, theme::INK, t.clamp(0.0, 1.0));
    let painter = ui.painter();
    painter.rect_filled(rect, theme::corner(rect.height() / 2.0), track);
    let pad = px(3.0);
    let thumb_r = rect.height() / 2.0 - pad;
    let x = egui::lerp(
        (rect.left() + pad + thumb_r)..=(rect.right() - pad - thumb_r),
        t.clamp(0.0, 1.0),
    );
    painter.circle_filled(
        egui::pos2(x, rect.center().y),
        thumb_r,
        theme::lerp_color(theme::INK_DIM, theme::PRIMARY_INK, t.clamp(0.0, 1.0)),
    );
    resp
}
