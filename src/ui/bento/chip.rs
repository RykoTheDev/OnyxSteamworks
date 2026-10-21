#![allow(dead_code)]

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Vec2};

use crate::ui::{anim, theme};

pub fn chip_size(ui: &egui::Ui, label: &str, s: f32) -> Vec2 {
    let px = |v: f32| theme::px(v, s);
    let font = FontId::new(px(12.0), theme::family_medium());
    let w = ui
        .painter()
        .layout_no_wrap(label.to_uppercase(), font, theme::INK)
        .size()
        .x;
    Vec2::new(w + px(12.0) * 2.0, px(26.0))
}

pub fn chip(ui: &mut egui::Ui, rect: Rect, label: &str, selected: bool, s: f32) -> Response {
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id(("chip", label, rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    let ctx = ui.ctx().clone();
    let t = anim::anim_bool(&ctx, id.with("sel"), selected, 0.2, anim::ease_out_cubic);
    let hover = anim::anim_bool(
        &ctx,
        id.with("hover"),
        resp.hovered(),
        0.14,
        anim::ease_out_cubic,
    );
    let r = anim::scale_rect(rect, 1.0 + 0.03 * hover);
    let radius = r.height() / 2.0;
    let painter = ui.painter();
    painter.rect_filled(
        r,
        theme::corner(radius),
        theme::lerp_color(theme::PANEL, theme::INK, t),
    );
    painter.rect_stroke(
        r,
        theme::corner(radius),
        Stroke::new(1.0, theme::INK),
        StrokeKind::Inside,
    );
    painter.text(
        r.center(),
        Align2::CENTER_CENTER,
        label.to_uppercase(),
        FontId::new(px(12.0), theme::family_medium()),
        theme::lerp_color(theme::INK, theme::PANEL, t),
    );
    resp
}

pub fn chip_grid(
    ui: &mut egui::Ui,
    rect: Rect,
    labels: &[&str],
    selected: &mut Vec<usize>,
    multi: bool,
    s: f32,
) -> bool {
    let px = |v: f32| theme::px(v, s);
    let gap = px(8.0);
    let row_h = px(26.0);
    let mut x = rect.left();
    let mut y = rect.top();
    let mut changed = false;
    for (i, label) in labels.iter().enumerate() {
        let size = chip_size(ui, label, s);
        if x + size.x > rect.right() && x > rect.left() {
            x = rect.left();
            y += row_h + gap;
        }
        let cr = Rect::from_min_size(Pos2::new(x, y), size);
        let is_sel = selected.contains(&i);
        if chip(ui, cr, label, is_sel, s).clicked() {
            changed = true;
            if multi {
                if let Some(pos) = selected.iter().position(|v| *v == i) {
                    selected.remove(pos);
                } else {
                    selected.push(i);
                }
            } else {
                *selected = vec![i];
            }
        }
        x += size.x + gap;
    }
    changed
}
