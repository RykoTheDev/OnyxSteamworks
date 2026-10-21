use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::card;
use super::state::StepRunStatus;
use crate::app::App;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind};
use crate::ui::theme;

enum Action {
    Toggle(usize),
    Expand(usize),
    Up(usize),
    Down(usize),
    Delete(usize),
    RunFrom(usize),
}

pub(super) fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.set_clip_rect(rect);
    let ui = &mut child;

    if app.pipeline_ui.draft.pipeline.is_empty() {
        kit::empty_state(
            ui,
            rect,
            theme::icon::FLOW,
            "No steps yet",
            "Add a step or apply an engine preset to build a pipeline.",
            s,
        );
        return;
    }

    let mut action = None;
    egui::ScrollArea::vertical()
        .id_salt("pipeline_timeline")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let count = app.pipeline_ui.draft.pipeline.len();
            for i in 0..count {
                if let Some(a) = card_row(app, ui, i, count, s) {
                    action = Some(a);
                }
                ui.add_space(px(10.0));
            }
        });

    match action {
        Some(Action::Toggle(i)) => {
            if let Some(step) = app.pipeline_ui.draft.pipeline.get_mut(i) {
                step.enabled = !step.enabled;
            }
        }
        Some(Action::Expand(i)) => {
            app.pipeline_ui.expanded = if app.pipeline_ui.expanded == Some(i) {
                None
            } else {
                Some(i)
            };
        }
        Some(Action::Up(i)) => app.pipeline_ui.move_step(i, true),
        Some(Action::Down(i)) => app.pipeline_ui.move_step(i, false),
        Some(Action::Delete(i)) => app.pipeline_ui.remove_step(i),
        Some(Action::RunFrom(i)) => app.pipeline_ui.start(ctx, i),
        None => {}
    }
}

fn card_row(app: &mut App, ui: &mut egui::Ui, i: usize, count: usize, s: f32) -> Option<Action> {
    let px = |v: f32| theme::px(v, s);
    let expanded = app.pipeline_ui.expanded == Some(i);
    let status = app.pipeline_ui.statuses.get(i).copied().unwrap_or_default();
    let kind = app.pipeline_ui.draft.pipeline[i].kind;
    let name = app.pipeline_ui.draft.pipeline[i].name.clone();
    let running = app.pipeline_ui.running;
    let extra = if expanded {
        px(card::extra_height(&app.pipeline_ui.draft.pipeline[i]))
    } else {
        0.0
    };
    let h = px(54.0) + extra;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_CARD),
    );

    let icon_c = Pos2::new(rect.left() + px(26.0), rect.top() + px(27.0));
    status_icon(ui, icon_c, px(9.0), status);
    if i + 1 < count {
        ui.painter().line_segment(
            [
                Pos2::new(icon_c.x, rect.bottom() - px(6.0)),
                Pos2::new(icon_c.x, rect.bottom() + px(10.0)),
            ],
            Stroke::new(1.0, theme::LINE),
        );
    }

    let tx = rect.left() + px(48.0);
    ui.painter().text(
        Pos2::new(tx, rect.top() + px(18.0)),
        Align2::LEFT_CENTER,
        &name,
        FontId::new(px(13.5), theme::family_semibold()),
        theme::INK,
    );
    ui.painter().text(
        Pos2::new(tx, rect.top() + px(36.0)),
        Align2::LEFT_CENTER,
        kind.label(),
        FontId::new(px(11.5), theme::family_medium()),
        theme::INK_MUTED,
    );

    let mut action = None;
    let by = rect.top() + px(10.0);
    let mut rx = rect.right() - px(10.0);
    let bw = px(34.0);
    let gap = px(6.0);

    rx -= bw;
    if kit::icon_button(
        ui,
        Rect::from_min_size(Pos2::new(rx, by), Vec2::new(bw, px(34.0))),
        theme::icon::TRASH,
        s,
    )
    .clicked()
    {
        action = Some(Action::Delete(i));
    }
    rx -= bw + gap;
    if kit::icon_button(
        ui,
        Rect::from_min_size(Pos2::new(rx, by), Vec2::new(bw, px(34.0))),
        theme::icon::CARET_DOWN,
        s,
    )
    .clicked()
    {
        action = Some(Action::Expand(i));
    }
    rx -= bw + gap;
    if Button::new("DN", ButtonKind::Secondary)
        .enabled(i + 1 < count)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(rx, by), Vec2::new(bw, px(34.0))),
            s,
        )
        .clicked()
    {
        action = Some(Action::Down(i));
    }
    rx -= bw + gap;
    if Button::new("UP", ButtonKind::Secondary)
        .enabled(i > 0)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(rx, by), Vec2::new(bw, px(34.0))),
            s,
        )
        .clicked()
    {
        action = Some(Action::Up(i));
    }
    rx -= bw + gap;
    if kit::icon_button(
        ui,
        Rect::from_min_size(Pos2::new(rx, by), Vec2::new(bw, px(34.0))),
        theme::icon::PLAY,
        s,
    )
    .clicked()
        && !running
    {
        action = Some(Action::RunFrom(i));
    }
    rx -= px(46.0) + gap;
    let mut enabled = app.pipeline_ui.draft.pipeline[i].enabled;
    if kit::toggle(
        ui,
        Rect::from_min_size(Pos2::new(rx, by + px(5.0)), Vec2::new(px(42.0), px(22.0))),
        &mut enabled,
        s,
    )
    .clicked()
    {
        action = Some(Action::Toggle(i));
    }

    if expanded {
        let inner = Rect::from_min_max(
            Pos2::new(rect.left() + px(48.0), rect.top() + px(56.0)),
            Pos2::new(rect.right() - px(16.0), rect.bottom() - px(10.0)),
        );
        card::fields(&mut app.pipeline_ui.draft.pipeline[i], ui, inner, s);
    }
    action
}

fn status_icon(ui: &mut egui::Ui, c: Pos2, r: f32, status: StepRunStatus) {
    let painter = ui.painter();
    match status {
        StepRunStatus::Idle | StepRunStatus::Pending => {
            painter.circle_stroke(c, r, Stroke::new(1.5, theme::INK_MUTED));
        }
        StepRunStatus::Running => {
            painter.circle_filled(c, r, theme::INK);
            painter.circle_stroke(
                c,
                r + 2.5,
                Stroke::new(1.0, theme::with_alpha(theme::INK, 80)),
            );
        }
        StepRunStatus::Success => {
            painter.circle_filled(c, r, theme::SUCCESS);
            check(painter, c, r, theme::PRIMARY_INK);
        }
        StepRunStatus::Failed => {
            painter.circle_filled(c, r, theme::DANGER);
            cross(painter, c, r * 0.5, theme::PRIMARY_INK);
        }
        StepRunStatus::Skipped => {
            painter.line_segment(
                [Pos2::new(c.x - r, c.y), Pos2::new(c.x + r, c.y)],
                Stroke::new(1.5, theme::INK_MUTED),
            );
        }
    }
}

fn check(painter: &egui::Painter, c: Pos2, r: f32, color: Color32) {
    let a = Pos2::new(c.x - r * 0.45, c.y + r * 0.05);
    let b = Pos2::new(c.x - r * 0.1, c.y + r * 0.4);
    let d = Pos2::new(c.x + r * 0.5, c.y - r * 0.4);
    painter.line_segment([a, b], Stroke::new(2.0, color));
    painter.line_segment([b, d], Stroke::new(2.0, color));
}

fn cross(painter: &egui::Painter, c: Pos2, r: f32, color: Color32) {
    painter.line_segment(
        [Pos2::new(c.x - r, c.y - r), Pos2::new(c.x + r, c.y + r)],
        Stroke::new(2.0, color),
    );
    painter.line_segment(
        [Pos2::new(c.x - r, c.y + r), Pos2::new(c.x + r, c.y - r)],
        Stroke::new(2.0, color),
    );
}
