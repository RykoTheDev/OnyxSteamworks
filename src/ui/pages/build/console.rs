use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Vec2};

use super::state::BuildState;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

pub fn show(state: &mut BuildState, ui: &mut egui::Ui, rect: Rect, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::BG,
        theme::LINE,
        px(theme::RADIUS_CARD),
    );
    let pad = px(12.0);
    let bar_h = px(36.0);
    let copy_w = px(96.0);
    let bar = Rect::from_min_size(
        Pos2::new(rect.left() + pad, rect.top() + pad),
        Vec2::new(rect.width() - pad * 2.0, bar_h),
    );
    let search_rect =
        Rect::from_min_size(bar.min, Vec2::new(bar.width() - copy_w - px(8.0), bar_h));
    Input::new(&mut state.search)
        .hint("Filter log")
        .show(ui, search_rect, s);
    if Button::new("COPY", ButtonKind::Secondary)
        .icon(theme::icon::COPY)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(bar.right() - copy_w, bar.top()),
                Vec2::new(copy_w, bar_h),
            ),
            s,
        )
        .clicked()
    {
        ctx.copy_text(state.log_text());
    }

    let log_rect = Rect::from_min_max(
        Pos2::new(rect.left() + pad, bar.bottom() + px(10.0)),
        Pos2::new(rect.right() - pad, rect.bottom() - pad),
    );
    let query = state.search.to_ascii_lowercase();
    let rows: Vec<(&str, bool)> = state
        .log
        .iter()
        .filter(|l| query.is_empty() || l.text.to_ascii_lowercase().contains(&query))
        .map(|l| (l.text.as_str(), l.error))
        .collect();

    let id = ui.make_persistent_id("build_console");
    let jump = ctx.data_mut(|d| d.get_temp::<bool>(id.with("jump")).unwrap_or(false));
    let row_h = px(16.0);
    let font = FontId::new(px(12.5), egui::FontFamily::Monospace);

    let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(log_rect));
    cui.set_clip_rect(log_rect);
    let mut sa = egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true);
    if jump {
        sa = sa.vertical_scroll_offset(f32::INFINITY);
        ctx.data_mut(|d| d.insert_temp(id.with("jump"), false));
    }
    let mut last_visible = 0usize;
    sa.show_rows(&mut cui, row_h, rows.len().max(1), |ui, range| {
        last_visible = range.end;
        if rows.is_empty() {
            let (r, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
            ui.painter().text(
                r.left_center(),
                Align2::LEFT_CENTER,
                "No output yet.",
                font.clone(),
                theme::INK_MUTED,
            );
            return;
        }
        for i in range {
            let (r, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
            if let Some((text, error)) = rows.get(i) {
                ui.painter().text(
                    r.left_center(),
                    Align2::LEFT_CENTER,
                    *text,
                    font.clone(),
                    if *error { theme::DANGER } else { theme::INK },
                );
            }
        }
    });

    let scrolled_up = !rows.is_empty() && last_visible < rows.len();
    if scrolled_up {
        let pill = Rect::from_min_size(
            Pos2::new(log_rect.right() - px(132.0), log_rect.bottom() - px(42.0)),
            Vec2::new(px(124.0), px(32.0)),
        );
        if Button::new("JUMP TO LATEST", ButtonKind::Secondary)
            .show(ui, pill, s)
            .clicked()
        {
            ctx.data_mut(|d| d.insert_temp(id.with("jump"), true));
        }
    }
}
