#![allow(dead_code)]

use eframe::egui::{
    self, Align2, FontId, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, epaint::Shape,
};

use crate::ui::{anim, theme};

pub fn dropdown(
    ui: &mut egui::Ui,
    rect: Rect,
    options: &[&str],
    selected: &mut usize,
    s: f32,
) -> egui::Response {
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id(("dropdown", rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    let ctx = ui.ctx().clone();
    let mut open = ctx.data(|d| d.get_temp::<bool>(id.with("open")).unwrap_or(false));
    if resp.clicked() {
        open = !open;
    }

    ui.painter()
        .rect_filled(rect, theme::corner(rect.height() / 2.0), theme::PANEL);
    ui.painter().rect_stroke(
        rect,
        theme::corner(rect.height() / 2.0),
        Stroke::new(1.0, theme::INK),
        StrokeKind::Inside,
    );
    let current = options.get(*selected).copied().unwrap_or("");
    ui.painter().text(
        Pos2::new(rect.left() + px(16.0), rect.center().y),
        Align2::LEFT_CENTER,
        current,
        FontId::new(px(13.0), theme::family_medium()),
        theme::INK,
    );
    chevron(
        ui,
        Pos2::new(rect.right() - px(18.0), rect.center().y),
        px(5.0),
        open,
    );

    let t = anim::anim_bool(&ctx, id.with("anim"), open, 0.14, anim::ease_out_cubic);
    if t > 0.001 {
        let row_h = px(32.0);
        let ph = row_h * options.len() as f32 + px(12.0);
        let pos = Pos2::new(rect.left(), rect.bottom() + px(6.0) - (1.0 - t) * px(6.0));
        let area = egui::Area::new(id.with("popup"))
            .order(Order::Foreground)
            .fixed_pos(pos)
            .show(&ctx, |ui| {
                ui.set_opacity(t);
                let prect = Rect::from_min_size(pos, Vec2::new(rect.width(), ph));
                ui.painter().rect_filled(
                    prect,
                    theme::corner(px(theme::RADIUS_POPUP)),
                    theme::PANEL,
                );
                for (i, opt) in options.iter().enumerate() {
                    let rr = Rect::from_min_size(
                        Pos2::new(
                            prect.left() + px(6.0),
                            prect.top() + px(6.0) + i as f32 * row_h,
                        ),
                        Vec2::new(prect.width() - px(12.0), row_h),
                    );
                    let r = ui.interact(rr, id.with(("opt", i)), Sense::click());
                    let hov = anim::anim_bool(
                        ui.ctx(),
                        id.with(("h", i)),
                        r.hovered(),
                        0.1,
                        anim::ease_out_cubic,
                    );
                    ui.painter().rect_filled(
                        rr,
                        theme::corner(px(12.0)),
                        theme::lerp_color(theme::PANEL, theme::PANEL_ALT, hov),
                    );
                    ui.painter().text(
                        Pos2::new(rr.left() + px(10.0), rr.center().y),
                        Align2::LEFT_CENTER,
                        *opt,
                        FontId::new(px(13.0), theme::family_medium()),
                        theme::INK,
                    );
                    if r.clicked() {
                        *selected = i;
                        open = false;
                    }
                }
            });
        if area.response.clicked_elsewhere() && !resp.clicked() {
            open = false;
        }
    }
    ctx.data_mut(|d| d.insert_temp(id.with("open"), open));
    resp
}

fn chevron(ui: &egui::Ui, center: Pos2, r: f32, up: bool) {
    let dir = if up { -1.0 } else { 1.0 };
    let pts = vec![
        center + Vec2::new(-r, -r * 0.5 * dir),
        center + Vec2::new(r, -r * 0.5 * dir),
        center + Vec2::new(0.0, r * 0.6 * dir),
    ];
    ui.painter()
        .add(Shape::convex_polygon(pts, theme::INK_MUTED, Stroke::NONE));
}
