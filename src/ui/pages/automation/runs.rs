use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Vec2};

use super::state::StepRunStatus;
use super::timeline;
use crate::app::App;
use crate::model::{PipelineRun, RunOutcome, StepOutcome};
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::pages::history::{fmt_duration, rel_time};
use crate::ui::{anim, theme};

const RUN_LIMIT: usize = 500;
const RELOAD_SECS: f64 = 15.0;

#[derive(Default)]
pub struct RunsState {
    runs: Vec<PipelineRun>,
    loaded: bool,
    loaded_at: f64,
    selected: Option<String>,
    step: Option<usize>,
}

impl RunsState {
    fn reload(&mut self) {
        self.runs = crate::history::connect()
            .and_then(|conn| crate::history::runs(&conn, RUN_LIMIT))
            .unwrap_or_else(|e| {
                eprintln!("could not load pipeline runs: {e}");
                Vec::new()
            });
        self.loaded = true;
        if self.selected.is_none() {
            self.select(0);
        }
    }

    fn select(&mut self, index: usize) {
        let Some(run) = self.runs.get(index) else {
            self.selected = None;
            return;
        };
        self.selected = Some(run.id.clone());
        self.step = run
            .steps
            .iter()
            .position(|s| s.outcome == StepOutcome::Failed)
            .or_else(|| {
                run.steps
                    .iter()
                    .rposition(|s| s.outcome != StepOutcome::NotRun)
            });
    }

    fn current(&self) -> Option<&PipelineRun> {
        let id = self.selected.as_ref()?;
        self.runs.iter().find(|r| &r.id == id)
    }
}

pub(super) fn outcome_color(outcome: RunOutcome) -> Color32 {
    match outcome {
        RunOutcome::Completed => theme::SUCCESS,
        RunOutcome::Failed => theme::DANGER,
        RunOutcome::Cancelled => theme::WARNING,
    }
}

fn outcome_label(outcome: RunOutcome) -> &'static str {
    match outcome {
        RunOutcome::Completed => "Completed",
        RunOutcome::Failed => "Failed",
        RunOutcome::Cancelled => "Cancelled",
    }
}

fn step_status(outcome: StepOutcome) -> StepRunStatus {
    match outcome {
        StepOutcome::NotRun => StepRunStatus::Idle,
        StepOutcome::Success => StepRunStatus::Success,
        StepOutcome::Failed => StepRunStatus::Failed,
        StepOutcome::Skipped => StepRunStatus::Skipped,
    }
}

pub(super) fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let now = ui.input(|i| i.time);
    let fresh = app.pipeline_ui.run_saved || app.scheduler.run_saved;
    if !app.runs_ui.loaded || fresh {
        app.pipeline_ui.run_saved = false;
        app.scheduler.run_saved = false;
        app.runs_ui.selected = None;
        app.runs_ui.reload();
        app.runs_ui.loaded_at = now;
    } else if now - app.runs_ui.loaded_at > RELOAD_SECS {
        app.runs_ui.reload();
        app.runs_ui.loaded_at = now;
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f64(RELOAD_SECS));
    if app.runs_ui.runs.is_empty() {
        kit::empty_state(
            ui,
            rect,
            theme::icon::HISTORY,
            "No pipeline runs yet",
            "Run a pipeline from Automation > Pipelines and its results show up here.",
            s,
        );
        return;
    }

    let list_w = px(380.0).min(rect.width() * 0.45);
    let list = Rect::from_min_size(rect.min, Vec2::new(list_w, rect.height()));
    let detail = Rect::from_min_max(Pos2::new(list.right() + px(20.0), rect.top()), rect.max);
    run_list(app, ui, list, s);
    run_detail(app, ui, detail, s);
}

fn run_list(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    cu.set_clip_rect(rect);
    let mut clicked = None;
    egui::ScrollArea::vertical()
        .id_salt("pipeline_runs_list")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            ui.set_width(rect.width() - px(12.0));
            for (i, run) in app.runs_ui.runs.iter().enumerate() {
                let (row, resp) = ui
                    .allocate_exact_size(Vec2::new(ui.available_width(), px(58.0)), Sense::click());
                let on = app.runs_ui.selected.as_ref() == Some(&run.id);
                let hover = anim::anim_bool(
                    ui.ctx(),
                    ui.make_persistent_id(("run_row", &run.id)),
                    resp.hovered(),
                    0.1,
                    anim::ease_out_cubic,
                );
                let fill = if on {
                    theme::FIELD_HI
                } else {
                    theme::with_alpha(theme::FIELD, (hover * 255.0) as u8)
                };
                ui.painter()
                    .rect_filled(row, theme::corner(px(theme::RADIUS_FIELD)), fill);
                let dot = Pos2::new(row.left() + px(16.0), row.top() + px(20.0));
                ui.painter()
                    .circle_filled(dot, px(4.0), outcome_color(run.outcome));
                ui.painter().text(
                    Pos2::new(row.left() + px(30.0), row.top() + px(20.0)),
                    Align2::LEFT_CENTER,
                    &run.pipeline_name,
                    FontId::new(px(13.0), theme::family_semibold()),
                    theme::INK,
                );
                ui.painter().text(
                    Pos2::new(row.left() + px(30.0), row.top() + px(40.0)),
                    Align2::LEFT_CENTER,
                    &run.project_name,
                    FontId::new(px(12.0), theme::family_medium()),
                    theme::INK_MUTED,
                );
                ui.painter().text(
                    Pos2::new(row.right() - px(12.0), row.top() + px(20.0)),
                    Align2::RIGHT_CENTER,
                    rel_time(run.started_ms),
                    FontId::new(px(12.0), theme::family_medium()),
                    theme::INK_MUTED,
                );
                ui.painter().text(
                    Pos2::new(row.right() - px(12.0), row.top() + px(40.0)),
                    Align2::RIGHT_CENTER,
                    fmt_duration(run.duration_ms),
                    FontId::new(px(12.0), theme::family_medium()),
                    theme::INK_FAINT,
                );
                if resp.clicked() {
                    clicked = Some(i);
                }
                ui.add_space(px(4.0));
            }
        });
    if let Some(i) = clicked {
        app.runs_ui.select(i);
    }
}

fn run_detail(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let Some(run) = app.runs_ui.current().cloned() else {
        return;
    };
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_CARD),
    );
    let x = rect.left() + px(20.0);
    let mut y = rect.top() + px(18.0);
    ui.painter().text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        &run.pipeline_name,
        FontId::new(px(16.0), theme::family_semibold()),
        theme::INK,
    );
    let summary = format!(
        "{}  ·  {}  ·  {}",
        run.project_name,
        rel_time(run.started_ms),
        fmt_duration(run.duration_ms)
    );
    ui.painter().text(
        Pos2::new(x, y + px(26.0)),
        Align2::LEFT_TOP,
        summary,
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK_MUTED,
    );
    let badge_font = FontId::new(px(12.0), theme::family_semibold());
    let label = outcome_label(run.outcome);
    let color = outcome_color(run.outcome);
    let w = ui
        .painter()
        .layout_no_wrap(label.to_string(), badge_font.clone(), color)
        .size()
        .x;
    let delete = Rect::from_min_size(
        Pos2::new(rect.right() - px(20.0) - px(110.0), y),
        Vec2::new(px(110.0), px(32.0)),
    );
    let badge = Rect::from_min_size(
        Pos2::new(delete.left() - px(12.0) - w - px(20.0), y + px(4.0)),
        Vec2::new(w + px(20.0), px(24.0)),
    );
    ui.painter().rect_filled(
        badge,
        theme::corner(badge.height() / 2.0),
        theme::with_alpha(color, 30),
    );
    ui.painter().text(
        badge.center(),
        Align2::CENTER_CENTER,
        label,
        badge_font,
        color,
    );
    if Button::new("Delete run", ButtonKind::Ghost)
        .icon(theme::icon::TRASH)
        .show(ui, delete, s)
        .clicked()
    {
        if let Err(e) =
            crate::history::connect().and_then(|c| crate::history::delete_run(&c, &run.id))
        {
            eprintln!("could not delete run: {e}");
        }
        app.runs_ui.selected = None;
        app.runs_ui.reload();
        return;
    }
    y += px(62.0);

    for (i, step) in run.steps.iter().enumerate() {
        let row = Rect::from_min_size(
            Pos2::new(rect.left() + px(10.0), y),
            Vec2::new(rect.width() - px(20.0), px(38.0)),
        );
        let resp = ui.interact(
            row,
            ui.make_persistent_id(("run_step", &run.id, i)),
            Sense::click(),
        );
        let on = app.runs_ui.step == Some(i);
        if on || resp.hovered() {
            ui.painter().rect_filled(
                row,
                theme::corner(px(theme::RADIUS_FIELD)),
                if on { theme::FIELD_HI } else { theme::FIELD },
            );
        }
        timeline::status_icon(
            ui,
            Pos2::new(row.left() + px(18.0), row.center().y),
            px(7.0),
            step_status(step.outcome),
        );
        ui.painter().text(
            Pos2::new(row.left() + px(36.0), row.center().y),
            Align2::LEFT_CENTER,
            &step.name,
            FontId::new(px(13.0), theme::family_medium()),
            if step.outcome == StepOutcome::NotRun {
                theme::INK_FAINT
            } else {
                theme::INK
            },
        );
        let right = if step.outcome == StepOutcome::NotRun {
            "Not run".to_string()
        } else {
            fmt_duration(step.duration_ms)
        };
        ui.painter().text(
            Pos2::new(row.right() - px(12.0), row.center().y),
            Align2::RIGHT_CENTER,
            right,
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
        if step.kind.label() != step.name {
            ui.painter().text(
                Pos2::new(row.right() - px(80.0), row.center().y),
                Align2::RIGHT_CENTER,
                step.kind.label(),
                FontId::new(px(12.0), theme::family_medium()),
                theme::INK_FAINT,
            );
        }
        if resp.clicked() {
            app.runs_ui.step = Some(i);
        }
        y += px(40.0);
    }

    let log_rect = Rect::from_min_max(
        Pos2::new(rect.left() + px(10.0), y + px(10.0)),
        Pos2::new(rect.right() - px(10.0), rect.bottom() - px(10.0)),
    );
    if log_rect.height() < px(60.0) {
        return;
    }
    kit::bordered(
        &ui.painter().clone(),
        log_rect,
        theme::BG,
        theme::LINE_SOFT,
        px(theme::RADIUS_FIELD),
    );
    let step = app.runs_ui.step.and_then(|i| run.steps.get(i));
    let title = step
        .map(|s| format!("{} log", s.name))
        .unwrap_or_else(|| "Select a step to see its log".to_string());
    ui.painter().text(
        Pos2::new(log_rect.left() + px(12.0), log_rect.top() + px(12.0)),
        Align2::LEFT_TOP,
        title,
        FontId::new(px(12.0), theme::family_label()),
        theme::INK_MUTED,
    );
    let inner = Rect::from_min_max(
        Pos2::new(log_rect.left() + px(12.0), log_rect.top() + px(34.0)),
        Pos2::new(log_rect.right() - px(8.0), log_rect.bottom() - px(8.0)),
    );
    let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    cu.set_clip_rect(inner);
    egui::ScrollArea::vertical()
        .id_salt(("pipeline_run_log", &run.id, app.runs_ui.step))
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(&mut cu, |ui| {
            let Some(step) = step else {
                return;
            };
            if step.log.is_empty() {
                ui.label(
                    egui::RichText::new("This step produced no output.")
                        .font(FontId::new(px(12.0), theme::family_medium()))
                        .color(theme::INK_MUTED),
                );
            }
            for line in &step.log {
                ui.label(
                    egui::RichText::new(line)
                        .monospace()
                        .size(px(11.5))
                        .color(theme::INK_DIM),
                );
            }
        });
}
