#![allow(dead_code)]

use eframe::egui::{self, Order, Rect, UiBuilder, Vec2};

use crate::ui::kit::overlay::should_dismiss;
use crate::ui::{anim, theme};

pub fn modal(
    ui: &mut egui::Ui,
    id: egui::Id,
    open: &mut bool,
    size: Vec2,
    s: f32,
    add: impl FnOnce(&mut egui::Ui) -> bool,
) -> egui::Response {
    modal_with(ui, id, open, size, s, true, add)
}

pub fn modal_with(
    ui: &mut egui::Ui,
    id: egui::Id,
    open: &mut bool,
    size: Vec2,
    s: f32,
    dismissible: bool,
    add: impl FnOnce(&mut egui::Ui) -> bool,
) -> egui::Response {
    let ctx = ui.ctx().clone();
    let t = anim::anim_bool(&ctx, id.with("m"), *open, 0.24, anim::ease_out_back);
    if !*open && t <= 0.001 {
        return ui.interact(Rect::ZERO, id.with("closed"), egui::Sense::hover());
    }

    let screen = ctx.content_rect();
    let area = egui::Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(screen.min)
        .show(&ctx, |ui| {
            let scale = 0.92 + 0.08 * t;
            let prect = Rect::from_center_size(screen.center(), size * scale);
            let dim = theme::with_alpha(theme::BG, (153.0 * t.clamp(0.0, 1.0)) as u8);
            ui.painter().rect_filled(screen, theme::corner(0.0), dim);
            let clicked = ui
                .interact(screen, id.with("dim"), egui::Sense::click())
                .clicked();
            let pointer = ui.input(|i| i.pointer.interact_pos());
            if should_dismiss(dismissible, clicked, prect, pointer) {
                *open = false;
            }
            ui.painter().rect_filled(
                prect,
                theme::corner(theme::px(theme::RADIUS_PANEL_MIN, s)),
                theme::PANEL,
            );
            let mut cu = ui.new_child(UiBuilder::new().max_rect(prect));
            cu.set_opacity(t.clamp(0.0, 1.0));
            if add(&mut cu) {
                *open = false;
            }
        });
    area.response
}

pub fn tooltip(response: egui::Response, text: &str) -> egui::Response {
    let text = text.to_owned();
    response.on_hover_ui(|ui| {
        ui.colored_label(theme::PANEL, text);
    })
}
