#![allow(dead_code)]

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use crate::app::App;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind};
use crate::ui::theme;
use crate::vdf::{BuildOptions, generate_vdfs};

pub(super) fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    let project = app.projects_ui.project();
    let files = generate_vdfs(&project, &BuildOptions::default());
    if app.projects_ui.vdf_file >= files.len() {
        app.projects_ui.vdf_file = 0;
    }

    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    let sel_rect = Rect::from_min_size(rect.min, Vec2::new(px(260.0), px(40.0)));
    kit::dropdown(ui, sel_rect, &names, &mut app.projects_ui.vdf_file, s);

    let idx = app.projects_ui.vdf_file.min(files.len().saturating_sub(1));
    let content = files.get(idx).map(|(_, c)| c.clone()).unwrap_or_default();

    let copy_rect = Rect::from_min_size(Pos2::new(rect.right() - px(110.0), rect.top()), Vec2::new(px(110.0), px(40.0)));
    if Button::new("COPY", ButtonKind::Secondary)
        .icon(theme::icon::COPY)
        .show(ui, copy_rect, s)
        .clicked()
    {
        ctx.copy_text(content.clone());
    }

    let block = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + px(52.0)), rect.max);
    kit::bordered(&ui.painter().clone(), block, theme::BG, theme::LINE, px(theme::RADIUS_CARD));
    let inner = block.shrink(px(16.0));
    let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(&mut cui, |ui| {
            for line in content.lines() {
                ui.label(
                    egui::RichText::new(line)
                        .monospace()
                        .size(px(12.0))
                        .color(theme::INK_DIM),
                );
            }
            if content.is_empty() {
                ui.painter().text(
                    inner.center(),
                    Align2::CENTER_CENTER,
                    "No depots to preview",
                    FontId::new(px(13.0), theme::family_medium()),
                    theme::INK_MUTED,
                );
            }
        });
}
