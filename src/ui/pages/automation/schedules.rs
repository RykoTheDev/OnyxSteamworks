use eframe::egui::{self, Align2, FontId, Pos2, Rect, UiBuilder, Vec2};

use super::runs::outcome_color;
use super::scheduler::ScheduleForm;
use crate::app::App;
use crate::features::schedule::{self, MissedPolicy, Recurrence, Schedule, TriggerKind, WEEKDAYS};
use crate::model::RunOutcome;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::pages::history::rel_time;
use crate::ui::theme;

enum RowAction {
    RunNow(String),
    Toggle(String, bool),
    Delete(String),
    Task(String, bool),
}

pub(super) fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    if app.projects.is_empty() {
        kit::empty_state(
            ui,
            rect,
            theme::icon::CLOCK,
            "No projects",
            "Create a project and a pipeline first, then schedule it here.",
            s,
        );
        return;
    }
    let mut top = rect.top();
    if app.scheduler.active.is_some() {
        top = active_banner(app, ui, rect, s) + px(12.0);
    }
    let form = Rect::from_min_size(
        Pos2::new(rect.left(), top),
        Vec2::new(rect.width(), px(240.0)),
    );
    new_schedule(app, ui, form, s);
    let list = Rect::from_min_max(Pos2::new(rect.left(), form.bottom() + px(18.0)), rect.max);
    if app.scheduler.schedules.is_empty() {
        kit::empty_state(
            ui,
            list,
            theme::icon::CLOCK,
            "No schedules yet",
            "Pick a pipeline and a time above to run it automatically.",
            s,
        );
    } else {
        rows(app, ui, list, s);
    }
}

fn active_banner(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) -> f32 {
    let px = |v: f32| theme::px(v, s);
    let Some(active) = &app.scheduler.active else {
        return rect.top();
    };
    let waiting = active.awaiting.clone();
    let banner = Rect::from_min_size(rect.min, Vec2::new(rect.width(), px(48.0)));
    let tint = if waiting.is_some() {
        theme::WARNING
    } else {
        theme::ACCENT
    };
    kit::bordered(
        &ui.painter().clone(),
        banner,
        theme::with_alpha(tint, 20),
        theme::with_alpha(tint, 120),
        px(theme::RADIUS_FIELD),
    );
    kit::spinner(
        &ui.painter().clone(),
        Pos2::new(banner.left() + px(20.0), banner.center().y),
        px(7.0),
        tint,
        ui.ctx(),
    );
    let text = match &waiting {
        Some(msg) => format!("{}: {msg}", active.title),
        None if active.step.is_empty() => format!("Running {}", active.title),
        None => format!("Running {}  ·  {}", active.title, active.step),
    };
    ui.painter().text(
        Pos2::new(banner.left() + px(38.0), banner.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::new(px(13.0), theme::family_medium()),
        theme::INK,
    );
    let cancel = Rect::from_min_size(
        Pos2::new(banner.right() - px(110.0), banner.top() + px(7.0)),
        Vec2::new(px(100.0), px(34.0)),
    );
    if Button::new("Cancel", ButtonKind::Danger)
        .show(ui, cancel, s)
        .clicked()
    {
        app.scheduler.cancel();
    }
    if waiting.is_some()
        && Button::new("Approve", ButtonKind::Primary)
            .icon(theme::icon::CHECK)
            .show(
                ui,
                Rect::from_min_size(
                    Pos2::new(cancel.left() - px(120.0), cancel.top()),
                    Vec2::new(px(110.0), px(34.0)),
                ),
                s,
            )
            .clicked()
    {
        app.scheduler.approve();
    }
    banner.bottom()
}

fn label(ui: &egui::Ui, pos: Pos2, text: &str, s: f32) {
    kit::label(&ui.painter().clone(), pos, text, s);
}

fn new_schedule(app: &mut App, ui: &mut egui::Ui, card: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        card,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_CARD),
    );
    let x = card.left() + px(18.0);
    let y = card.top() + px(16.0);
    kit::heading(&ui.painter().clone(), Pos2::new(x, y), "New schedule", s);

    let row1 = y + px(32.0);
    let names: Vec<String> = app
        .projects
        .iter()
        .map(|p| {
            if p.name.is_empty() {
                "Untitled".to_string()
            } else {
                p.name.clone()
            }
        })
        .collect();
    let refs: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
    let form = &mut app.scheduler.form;
    form.project_sel = form.project_sel.min(app.projects.len() - 1);
    label(ui, Pos2::new(x, row1), "Project", s);
    let before = form.project_sel;
    kit::dropdown(
        ui,
        Rect::from_min_size(
            Pos2::new(x, row1 + px(20.0)),
            Vec2::new(px(200.0), px(36.0)),
        ),
        &refs,
        &mut form.project_sel,
        s,
    );
    if form.project_sel != before {
        form.pipeline_sel = 0;
    }
    let project = &app.projects[form.project_sel];
    let pipe_names: Vec<&str> = if project.pipelines.is_empty() {
        vec!["No pipelines"]
    } else {
        project.pipelines.iter().map(|p| p.name.as_str()).collect()
    };
    let has_pipelines = !project.pipelines.is_empty();
    form.pipeline_sel = form.pipeline_sel.min(pipe_names.len() - 1);
    let px2 = x + px(212.0);
    label(ui, Pos2::new(px2, row1), "Pipeline", s);
    kit::dropdown(
        ui,
        Rect::from_min_size(
            Pos2::new(px2, row1 + px(20.0)),
            Vec2::new(px(200.0), px(36.0)),
        ),
        &pipe_names,
        &mut form.pipeline_sel,
        s,
    );

    let content_hint = crate::features::maintenance::resolve(project)
        .content_root
        .to_string_lossy()
        .to_string();
    let repo_hint = crate::features::patchnotes::repo_path(project)
        .to_string_lossy()
        .to_string();
    let tx0 = px2 + px(224.0);
    label(ui, Pos2::new(tx0, row1), "Start", s);
    let mut cx = tx0;
    for kind in TriggerKind::ALL {
        let sz = kit::chip_size(ui, kind.label(), s);
        let r = Rect::from_min_size(Pos2::new(cx, row1 + px(24.0)), sz);
        if kit::chip(ui, r, kind.label(), form.trigger == kind, s).clicked() {
            form.trigger = kind;
        }
        cx += sz.x + px(8.0);
    }

    let row2 = row1 + px(70.0);
    match form.trigger {
        TriggerKind::Time => time_row(form, ui, card, x, row2, s),
        TriggerKind::FolderChange => {
            let w = card.right() - px(18.0) - x - px(250.0);
            label(ui, Pos2::new(x, row2), "Folder to watch", s);
            Input::new(&mut form.watch_path).hint(&content_hint).show(
                ui,
                Rect::from_min_size(Pos2::new(x, row2 + px(20.0)), Vec2::new(w, px(36.0))),
                s,
            );
            let qx = x + w + px(24.0);
            label(ui, Pos2::new(qx, row2), "Quiet period (minutes)", s);
            stepper(
                ui,
                Pos2::new(qx, row2 + px(24.0)),
                &mut form.quiet_minutes,
                5,
                120,
                5,
                s,
            );
        }
        TriggerKind::GitTag => {
            label(ui, Pos2::new(x, row2), "Tag pattern", s);
            Input::new(&mut form.tag_pattern).hint("v*").show(
                ui,
                Rect::from_min_size(
                    Pos2::new(x, row2 + px(20.0)),
                    Vec2::new(px(180.0), px(36.0)),
                ),
                s,
            );
            let rx = x + px(196.0);
            label(ui, Pos2::new(rx, row2), "Repository folder", s);
            Input::new(&mut form.watch_path).hint(&repo_hint).show(
                ui,
                Rect::from_min_size(
                    Pos2::new(rx, row2 + px(20.0)),
                    Vec2::new(card.right() - px(18.0) - rx, px(36.0)),
                ),
                s,
            );
        }
    }

    let row3 = card.bottom() - px(48.0);
    if cfg!(windows) {
        kit::checkbox(
            ui,
            Rect::from_min_size(Pos2::new(x, row3), Vec2::new(px(420.0), px(34.0))),
            &mut form.use_task,
            "Run even when Onyx is closed (Windows Task Scheduler)",
            s,
        );
    }
    let add = Rect::from_min_size(
        Pos2::new(card.right() - px(18.0) - px(150.0), row3),
        Vec2::new(px(150.0), px(34.0)),
    );
    if Button::new("Add schedule", ButtonKind::Primary)
        .icon(theme::icon::PLUS)
        .enabled(has_pipelines)
        .disabled_reason("Create a pipeline in Automation > Pipelines first")
        .show(ui, add, s)
        .clicked()
    {
        match app.scheduler.add(&app.projects) {
            Ok(msg) => app.toasts.push(kit::ToastLevel::Success, msg),
            Err(e) => app.toasts.push(kit::ToastLevel::Error, e),
        }
    }
}

fn time_row(form: &mut ScheduleForm, ui: &mut egui::Ui, card: Rect, x: f32, row2: f32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    label(ui, Pos2::new(x, row2), "Repeat", s);
    let mut rx = x;
    for rec in Recurrence::ALL {
        let sz = kit::chip_size(ui, rec.label(), s);
        let r = Rect::from_min_size(Pos2::new(rx, row2 + px(24.0)), sz);
        if kit::chip(ui, r, rec.label(), form.recurrence == rec, s).clicked() {
            form.recurrence = rec;
        }
        rx += sz.x + px(8.0);
    }

    let tx = x + px(220.0);
    label(ui, Pos2::new(tx, row2), "Time", s);
    stepper(
        ui,
        Pos2::new(tx, row2 + px(24.0)),
        &mut form.hour,
        0,
        23,
        1,
        s,
    );
    ui.painter().text(
        Pos2::new(tx + px(103.0), row2 + px(38.0)),
        Align2::CENTER_CENTER,
        ":",
        FontId::new(px(14.0), theme::family_semibold()),
        theme::INK_MUTED,
    );
    stepper(
        ui,
        Pos2::new(tx + px(112.0), row2 + px(24.0)),
        &mut form.minute,
        0,
        55,
        5,
        s,
    );

    let dx = tx + px(236.0);
    match form.recurrence {
        Recurrence::Weekly => {
            label(ui, Pos2::new(dx, row2), "Day", s);
            let mut wx = dx;
            for (i, wd) in WEEKDAYS.iter().enumerate() {
                let sz = kit::chip_size(ui, wd, s);
                let r = Rect::from_min_size(Pos2::new(wx, row2 + px(24.0)), sz);
                if kit::chip(ui, r, wd, form.weekday == i as u32, s).clicked() {
                    form.weekday = i as u32;
                }
                wx += sz.x + px(6.0);
            }
        }
        Recurrence::Once => {
            label(ui, Pos2::new(dx, row2), "Date (day / month / year)", s);
            stepper(
                ui,
                Pos2::new(dx, row2 + px(24.0)),
                &mut form.day,
                1,
                31,
                1,
                s,
            );
            stepper(
                ui,
                Pos2::new(dx + px(108.0), row2 + px(24.0)),
                &mut form.month,
                1,
                12,
                1,
                s,
            );
            let mut year = form.year as u32;
            stepper(
                ui,
                Pos2::new(dx + px(216.0), row2 + px(24.0)),
                &mut year,
                2024,
                2099,
                1,
                s,
            );
            form.year = year as i64;
        }
        Recurrence::Daily => {
            ui.painter().text(
                Pos2::new(dx, row2 + px(38.0)),
                Align2::LEFT_CENTER,
                "Every day, local time",
                FontId::new(px(12.5), theme::family_medium()),
                theme::INK_MUTED,
            );
        }
    }

    let mx = card.right() - px(18.0) - px(180.0);
    label(ui, Pos2::new(mx, row2), "If a run is missed", s);
    let mut missed_sel = MissedPolicy::ALL
        .iter()
        .position(|m| *m == form.missed)
        .unwrap_or(0);
    let missed_labels: Vec<&str> = MissedPolicy::ALL.iter().map(|m| m.label()).collect();
    kit::dropdown(
        ui,
        Rect::from_min_size(
            Pos2::new(mx, row2 + px(20.0)),
            Vec2::new(px(180.0), px(36.0)),
        ),
        &missed_labels,
        &mut missed_sel,
        s,
    );
    form.missed = MissedPolicy::ALL[missed_sel];
}

fn stepper(ui: &mut egui::Ui, at: Pos2, value: &mut u32, min: u32, max: u32, step: u32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let minus = Rect::from_min_size(at, Vec2::splat(px(28.0)));
    let plus = Rect::from_min_size(
        Pos2::new(minus.right() + px(38.0), at.y),
        Vec2::splat(px(28.0)),
    );
    ui.painter().text(
        Pos2::new((minus.right() + plus.left()) / 2.0, at.y + px(14.0)),
        Align2::CENTER_CENTER,
        if max >= 1000 {
            value.to_string()
        } else {
            format!("{value:02}")
        },
        FontId::new(px(13.5), theme::family_semibold()),
        theme::INK,
    );
    if Button::new("-", ButtonKind::Secondary)
        .show(ui, minus, s)
        .clicked()
    {
        *value = if *value <= min {
            max - (max - min) % step
        } else {
            value.saturating_sub(step).max(min)
        };
    }
    if Button::new("+", ButtonKind::Secondary)
        .show(ui, plus, s)
        .clicked()
    {
        *value = if *value + step > max {
            min
        } else {
            *value + step
        };
    }
}

fn subtitle(sch: &Schedule, error: Option<&String>) -> String {
    if sch.is_legacy() {
        return "Created before schedules ran pipelines. Delete it and add a new one.".into();
    }
    if let Some(e) = error {
        return format!("Can't check this trigger: {e}");
    }
    let mut parts = vec![sch.describe()];
    if !sch.enabled {
        parts.push("Paused".into());
    } else if !sch.is_event() {
        parts.push(format!("Next {}", schedule::fmt_local(sch.next_ms)));
    } else if !sch.last_trigger_note.is_empty() {
        parts.push(format!("Last triggered by {}", sch.last_trigger_note));
    } else {
        parts.push("Watching".into());
    }
    if let Some(outcome) = sch.last_outcome {
        let word = match outcome {
            RunOutcome::Completed => "completed",
            RunOutcome::Failed => "failed",
            RunOutcome::Cancelled => "cancelled",
        };
        parts.push(format!(
            "Last run {word} {}",
            rel_time(sch.last_finished_ms)
        ));
    }
    parts.join("  ·  ")
}

fn rows(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let schedules = app.scheduler.schedules.clone();
    let trigger_errors = app.scheduler.trigger_errors.clone();
    let running = app.scheduler.active.as_ref().map(|a| a.schedule_id.clone());
    let busy = app.scheduler.busy() || app.pipeline_ui.running;
    let mut action = None;
    let mut cu = ui.new_child(UiBuilder::new().max_rect(rect));
    cu.set_clip_rect(rect);
    egui::ScrollArea::vertical()
        .id_salt("schedule_list")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            ui.set_width(rect.width() - px(12.0));
            for sch in &schedules {
                let (r, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), px(64.0)),
                    egui::Sense::hover(),
                );
                kit::bordered(
                    ui.painter(),
                    r,
                    theme::SURFACE_2,
                    theme::LINE,
                    px(theme::RADIUS_CARD),
                );
                let dot_color = if running.as_ref() == Some(&sch.id) {
                    theme::ACCENT
                } else if !sch.enabled || sch.is_legacy() {
                    theme::INK_FAINT
                } else {
                    sch.last_outcome.map(outcome_color).unwrap_or(theme::SUCCESS)
                };
                ui.painter().circle_filled(
                    Pos2::new(r.left() + px(18.0), r.top() + px(22.0)),
                    px(4.0),
                    dot_color,
                );
                let title = if sch.is_legacy() {
                    format!("Build  ·  {}", sch.project_name)
                } else {
                    format!("{}  ·  {}", sch.pipeline_name, sch.project_name)
                };
                ui.painter().text(
                    Pos2::new(r.left() + px(32.0), r.top() + px(22.0)),
                    Align2::LEFT_CENTER,
                    title,
                    FontId::new(px(13.5), theme::family_semibold()),
                    theme::INK,
                );
                let sub_font = FontId::new(px(12.0), theme::family_medium());
                let controls = if sch.is_legacy() { px(60.0) } else { px(420.0) };
                let sub = kit::input::head_fit(
                    ui.painter(),
                    &subtitle(sch, trigger_errors.get(&sch.id)),
                    &sub_font,
                    (r.width() - px(32.0) - controls).max(px(80.0)),
                );
                ui.painter().text(
                    Pos2::new(r.left() + px(32.0), r.top() + px(44.0)),
                    Align2::LEFT_CENTER,
                    sub,
                    sub_font,
                    if sch.is_legacy() || trigger_errors.contains_key(&sch.id) {
                        theme::WARNING
                    } else {
                        theme::INK_MUTED
                    },
                );

                let bw = px(32.0);
                let trash = Rect::from_min_size(
                    Pos2::new(r.right() - px(12.0) - bw, r.center().y - bw / 2.0),
                    Vec2::splat(bw),
                );
                if kit::tooltip(kit::icon_button(ui, trash, theme::icon::TRASH, s), "Delete schedule")
                    .clicked()
                {
                    action = Some(RowAction::Delete(sch.id.clone()));
                }
                if sch.is_legacy() {
                    ui.add_space(px(10.0));
                    continue;
                }
                let toggle = Rect::from_min_size(
                    Pos2::new(trash.left() - px(54.0), r.center().y - px(11.0)),
                    Vec2::new(px(42.0), px(22.0)),
                );
                let mut on = sch.enabled;
                if kit::tooltip(kit::toggle(ui, toggle, &mut on, s), "Pause or resume").changed() {
                    action = Some(RowAction::Toggle(sch.id.clone(), on));
                }
                let run = Rect::from_min_size(
                    Pos2::new(toggle.left() - px(16.0) - px(108.0), r.center().y - px(17.0)),
                    Vec2::new(px(108.0), px(34.0)),
                );
                if Button::new("Run now", ButtonKind::Secondary)
                    .icon(theme::icon::PLAY)
                    .enabled(!busy)
                    .disabled_reason("Another pipeline is running")
                    .show(ui, run, s)
                    .clicked()
                {
                    action = Some(RowAction::RunNow(sch.id.clone()));
                }
                let (badge_text, badge_tip) = if sch.task_installed {
                    (
                        "Task Scheduler",
                        "Windows runs this even when Onyx is closed. Click to run it only while Onyx is open.",
                    )
                } else {
                    (
                        "While Onyx is open",
                        "Runs only while Onyx is open. Click to register it with Windows Task Scheduler.",
                    )
                };
                let font = FontId::new(px(11.5), theme::family_medium());
                let w = ui
                    .painter()
                    .layout_no_wrap(badge_text.to_string(), font.clone(), theme::INK)
                    .size()
                    .x;
                let badge = Rect::from_min_size(
                    Pos2::new(run.left() - px(12.0) - w - px(20.0), r.center().y - px(12.0)),
                    Vec2::new(w + px(20.0), px(24.0)),
                );
                let resp = ui.interact(
                    badge,
                    ui.make_persistent_id(("sched_badge", &sch.id)),
                    egui::Sense::click(),
                );
                let tint = if sch.task_installed {
                    theme::ACCENT
                } else {
                    theme::INK_MUTED
                };
                ui.painter().rect_filled(
                    badge,
                    theme::corner(badge.height() / 2.0),
                    theme::with_alpha(tint, if resp.hovered() { 46 } else { 26 }),
                );
                ui.painter().text(
                    badge.center(),
                    Align2::CENTER_CENTER,
                    badge_text,
                    font,
                    if sch.task_installed {
                        theme::ACCENT_HI
                    } else {
                        theme::INK_DIM
                    },
                );
                if cfg!(windows)
                    && kit::tooltip(resp, badge_tip).clicked()
                {
                    action = Some(RowAction::Task(sch.id.clone(), !sch.task_installed));
                }
                ui.add_space(px(10.0));
            }
        });

    let ctx = ui.ctx().clone();
    match action {
        Some(RowAction::RunNow(id)) => match app.scheduler.run_now(&id, &app.projects, &ctx) {
            Ok(()) => app.toasts.push(kit::ToastLevel::Success, "Started"),
            Err(e) => app.toasts.push(kit::ToastLevel::Error, e),
        },
        Some(RowAction::Toggle(id, on)) => app.scheduler.set_enabled(&id, on),
        Some(RowAction::Delete(id)) => {
            app.scheduler.remove(&id);
            app.toasts
                .push(kit::ToastLevel::Success, "Schedule deleted");
        }
        Some(RowAction::Task(id, install)) => app.scheduler.set_task(&id, install),
        None => {}
    }
}
