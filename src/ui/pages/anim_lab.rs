use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Vec2};

use crate::app::App;
use crate::ui::{anim, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let _ = app;
    let ctx = ui.ctx().clone();
    let s = theme::scale(ui.available_size());
    let px = |v: f32| theme::px(v, s);

    let area = ui.max_rect();
    let pad = px(theme::SPACE_LG);
    let painter = ui.painter().clone();

    painter.text(
        area.left_top() + Vec2::new(pad, pad),
        Align2::LEFT_TOP,
        "ANIMATION LAB",
        FontId::new(px(16.0), theme::family_semibold()),
        theme::PANEL,
    );

    let mut y = area.top() + pad + px(40.0);
    let left = area.left() + pad;
    let col_w = (area.width() - pad * 2.0).min(px(760.0));

    // 1) Hover-scale card.
    y = section_label(&painter, left, y, px(12.0), "HOVER + PRESS");
    let card = Rect::from_min_size(Pos2::new(left, y), Vec2::new(px(220.0), px(120.0)));
    let card_id = ui.scope_id().with("hover_card");
    let resp = ui.interact(card, card_id, Sense::click());
    let hover = anim::anim_bool(&ctx, card_id, resp.hovered(), 0.14, anim::ease_out_cubic);
    let press = anim::anim_bool(
        &ctx,
        card_id.with("press"),
        resp.is_pointer_button_down_on(),
        0.09,
        anim::ease_out_cubic,
    );
    let scale = 1.0 + 0.03 * hover - 0.06 * press;
    let scaled = anim::scale_rect(card, scale);
    painter.rect_filled(scaled, theme::corner(px(theme::RADIUS_CARD)), theme::PANEL);
    painter.text(
        scaled.center(),
        Align2::CENTER_CENTER,
        "HOVER ME",
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK,
    );
    y += px(120.0) + pad;

    // 2) Spring-popping circle (click to pop).
    y = section_label(&painter, left, y, px(12.0), "SPRING (CLICK)");
    let zone = Rect::from_min_size(Pos2::new(left, y), Vec2::new(col_w, px(140.0)));
    let zresp = ui.interact(zone, ui.scope_id().with("spring_zone"), Sense::click());
    let key = ui.scope_id().with("spring_start");
    if zresp.clicked() {
        let now = ctx.input(|i| i.time);
        ctx.data_mut(|d| d.insert_temp(key, now));
    }
    let start = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(key, || f64::NEG_INFINITY));
    let scale = if start.is_finite() {
        let elapsed = (ctx.input(|i| i.time) - start) as f32;
        let settle = anim::spring(elapsed, 14.0);
        if settle < 0.999 {
            ctx.request_repaint();
        }
        1.0 + 0.5 * (1.0 - settle)
    } else {
        1.0
    };
    painter.rect_filled(zone, theme::corner(px(theme::RADIUS_CARD)), theme::PANEL);
    painter.circle_filled(zone.center(), px(34.0) * scale, theme::INK);
    y += px(140.0) + pad;

    // 3) Staggered list (click to replay).
    y = section_label(&painter, left, y, px(12.0), "STAGGER (CLICK TO REPLAY)");
    let group = ui.scope_id().with("stagger_group");
    let list = Rect::from_min_size(Pos2::new(left, y), Vec2::new(col_w, px(180.0)));
    let lresp = ui.interact(list, group.with("replay"), Sense::click());
    if lresp.clicked() {
        anim::reset_stagger(&ctx, group);
    }
    let row_h = px(26.0);
    let gap = px(6.0);
    for i in 0..5usize {
        let (opacity, y_ref) = anim::stagger_in(&ctx, group, i, 70.0);
        let row = Rect::from_min_size(
            Pos2::new(
                list.left(),
                list.top() + i as f32 * (row_h + gap) + px(y_ref),
            ),
            Vec2::new(col_w, row_h),
        );
        painter.rect_filled(
            row,
            theme::corner(px(12.0)),
            anim::with_opacity(theme::PANEL, opacity),
        );
    }

    let toggle = Rect::from_min_size(
        Pos2::new(left, area.bottom() - pad - px(26.0)),
        Vec2::new(px(220.0), px(26.0)),
    );
    let tresp = ui.interact(toggle, ui.scope_id().with("reduce_motion"), Sense::click());
    if tresp.clicked() {
        anim::set_reduce_motion(!anim::reduce_motion());
    }
    let state = if anim::reduce_motion() { "ON" } else { "OFF" };
    painter.rect_stroke(
        toggle,
        theme::corner(px(13.0)),
        egui::Stroke::new(1.0, theme::PANEL),
        egui::StrokeKind::Inside,
    );
    painter.text(
        toggle.center(),
        Align2::CENTER_CENTER,
        format!("REDUCE MOTION: {state}"),
        FontId::new(px(12.0), theme::family_medium()),
        theme::PANEL,
    );
}

fn section_label(painter: &egui::Painter, x: f32, y: f32, size: f32, text: &str) -> f32 {
    painter.text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        text,
        FontId::new(size, theme::family_medium()),
        theme::INK_MUTED,
    );
    y + size + 6.0
}
