use eframe::egui::{self, Align2, FontId, Pos2, Rect, UiBuilder, Vec2};

use crate::app::App;
use crate::features::schedule::{self, Recurrence, Schedule};
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    app.queue.form.seed(schedule::now_ms());
    if app.projects.is_empty() {
        kit::empty_state(
            ui,
            rect,
            theme::icon::CLOCK,
            "No projects",
            "Create a project first, then schedule its runs.",
            s,
        );
        return;
    }
    add_form(app, ui, rect, s);
    let list = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + px(150.0)), rect.max);
    if app.queue.schedules.is_empty() {
        kit::empty_state(
            ui,
            list,
            theme::icon::CLOCK,
            "No schedule yet",
            "Add a run above; it fires while the app is open.",
            s,
        );
    } else {
        rows(app, ui, list, s);
    }
}

fn add_form(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let card = Rect::from_min_size(rect.min, Vec2::new(rect.width(), px(132.0)));
    kit::bordered(
        &ui.painter().clone(),
        card,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    let x = card.left() + px(18.0);

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
    let refs: Vec<&str> = names.iter().map(|x| x.as_str()).collect();
    let dd = Rect::from_min_size(
        Pos2::new(x, card.top() + px(16.0)),
        Vec2::new(px(200.0), px(32.0)),
    );
    kit::dropdown(ui, dd, &refs, &mut app.queue.form.project_sel, s);

    let mut rx = dd.right() + px(12.0);
    for (label, rec) in [
        ("Once", Recurrence::Once),
        ("Daily", Recurrence::Daily),
        ("Weekly", Recurrence::Weekly),
    ] {
        let sz = kit::chip_size(ui, label, s);
        let r = Rect::from_min_size(Pos2::new(rx, dd.top() + px(4.0)), sz);
        if kit::chip(ui, r, label, app.queue.form.recurrence == rec, s).clicked() {
            app.queue.form.recurrence = rec;
        }
        rx += sz.x + px(8.0);
    }

    let y2 = card.top() + px(60.0);
    stepper(ui, Pos2::new(x, y2), "H", &mut app.queue.form.hour, 23, s);
    stepper(
        ui,
        Pos2::new(x + px(96.0), y2),
        "M",
        &mut app.queue.form.minute,
        59,
        s,
    );
    let mut vx = x + px(200.0);
    match app.queue.form.recurrence {
        Recurrence::Once => {
            stepper_wide(ui, Pos2::new(vx, y2), "Year", &mut app.queue.form.year, s);
            vx += px(132.0);
            let mut mo = app.queue.form.month;
            stepper(ui, Pos2::new(vx, y2), "Mon", &mut mo, 12, s);
            app.queue.form.month = mo.max(1);
            vx += px(96.0);
            let mut dy = app.queue.form.day;
            stepper(ui, Pos2::new(vx, y2), "Day", &mut dy, 31, s);
            app.queue.form.day = dy.max(1);
        }
        Recurrence::Weekly => {
            for (i, wd) in WEEKDAYS.iter().enumerate() {
                let sz = kit::chip_size(ui, wd, s);
                let r = Rect::from_min_size(Pos2::new(vx, y2 + px(4.0)), sz);
                if kit::chip(ui, r, wd, app.queue.form.weekday == i as u32, s).clicked() {
                    app.queue.form.weekday = i as u32;
                }
                vx += sz.x + px(6.0);
            }
        }
        Recurrence::Daily => {
            ui.painter().text(
                Pos2::new(vx, y2 + px(15.0)),
                Align2::LEFT_CENTER,
                "every day at this time (UTC)",
                FontId::new(px(12.0), theme::family_medium()),
                theme::INK_MUTED,
            );
        }
    }

    let add = Rect::from_min_size(
        Pos2::new(card.right() - px(150.0), card.bottom() - px(48.0)),
        Vec2::new(px(132.0), px(34.0)),
    );
    if Button::new("ADD SCHEDULE", ButtonKind::Primary)
        .icon(theme::icon::PLUS)
        .show(ui, add, s)
        .clicked()
    {
        add_schedule(app);
    }
}

fn add_schedule(app: &mut App) {
    let i = app.queue.form.project_sel.min(app.projects.len() - 1);
    let project = &app.projects[i];
    let branch = project
        .branches
        .first()
        .map(|b| b.name.clone())
        .unwrap_or_default();
    let schedule = Schedule {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project.id.clone(),
        project_name: if project.name.is_empty() {
            "Untitled".to_string()
        } else {
            project.name.clone()
        },
        branch,
        preview: false,
        recurrence: app.queue.form.recurrence,
        next_ms: app.queue.form.next_ms(),
        enabled: true,
    };
    app.queue.schedules.push(schedule);
    schedule::save(&app.queue.schedules);
    app.toasts.push(kit::ToastLevel::Success, "Schedule added");
}

fn rows(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let schedules = app.queue.schedules.clone();
    let mut toggle = None;
    let mut remove = None;
    let mut install = None;
    let mut uninstall = None;
    let mut cu = ui.new_child(UiBuilder::new().max_rect(rect));
    cu.set_clip_rect(rect);
    egui::ScrollArea::vertical()
        .id_salt("schedule_list")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            ui.set_width(rect.width() - px(14.0));
            for (i, sch) in schedules.iter().enumerate() {
                let (r, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), px(60.0)),
                    egui::Sense::hover(),
                );
                kit::bordered(
                    ui.painter(),
                    r,
                    theme::SURFACE_2,
                    theme::LINE,
                    px(theme::RADIUS_CARD),
                );
                let dot = if sch.enabled {
                    theme::SUCCESS
                } else {
                    theme::INK_MUTED
                };
                ui.painter().circle_filled(
                    Pos2::new(r.left() + px(16.0), r.center().y - px(8.0)),
                    px(4.0),
                    dot,
                );
                ui.painter().text(
                    Pos2::new(r.left() + px(28.0), r.center().y - px(10.0)),
                    Align2::LEFT_CENTER,
                    &sch.project_name,
                    FontId::new(px(13.5), theme::family_semibold()),
                    theme::INK,
                );
                let sub = format!(
                    "{}  ·  next {}",
                    sch.recurrence.label(),
                    schedule::fmt_dt(sch.next_ms)
                );
                ui.painter().text(
                    Pos2::new(r.left() + px(28.0), r.center().y + px(10.0)),
                    Align2::LEFT_CENTER,
                    sub,
                    FontId::new(px(11.5), theme::family_medium()),
                    theme::INK_MUTED,
                );

                let bw = px(30.0);
                let dz = Rect::from_min_size(
                    Pos2::new(r.right() - px(12.0) - bw, r.center().y - bw / 2.0),
                    Vec2::splat(bw),
                );
                let tz = Rect::from_min_size(
                    Pos2::new(dz.left() - px(50.0), r.center().y - px(12.0)),
                    Vec2::new(px(44.0), px(24.0)),
                );
                let mut en = sch.enabled;
                if kit::toggle(ui, tz, &mut en, s).changed() {
                    toggle = Some((i, en));
                }
                let task = Rect::from_min_size(
                    Pos2::new(tz.left() - px(10.0) - px(118.0), r.center().y - px(16.0)),
                    Vec2::new(px(118.0), px(32.0)),
                );
                if Button::new("INSTALL TASK", ButtonKind::Secondary)
                    .enabled(cfg!(windows))
                    .disabled_reason("Windows only")
                    .show(ui, task, s)
                    .clicked()
                {
                    install = Some(i);
                }
                let rmtask = Rect::from_min_size(
                    Pos2::new(task.left() - px(8.0) - px(110.0), r.center().y - px(16.0)),
                    Vec2::new(px(110.0), px(32.0)),
                );
                if Button::new("REMOVE TASK", ButtonKind::Secondary)
                    .enabled(cfg!(windows))
                    .disabled_reason("Windows only")
                    .show(ui, rmtask, s)
                    .clicked()
                {
                    uninstall = Some(i);
                }
                if kit::icon_button(ui, dz, theme::icon::TRASH, s).clicked() {
                    remove = Some(i);
                }
                ui.add_space(px(10.0));
            }
        });
    if let Some((i, en)) = toggle {
        app.queue.schedules[i].enabled = en;
        schedule::save(&app.queue.schedules);
    }
    if let Some(i) = install {
        run_schtasks(app, i, true);
    }
    if let Some(i) = uninstall {
        run_schtasks(app, i, false);
    }
    if let Some(i) = remove {
        app.queue.schedules.remove(i);
        schedule::save(&app.queue.schedules);
        app.toasts
            .push(kit::ToastLevel::Success, "Schedule removed");
    }
}

fn run_schtasks(app: &mut App, i: usize, install: bool) {
    let sch = app.queue.schedules[i].clone();
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "onyx".to_string());
    let reference = if sch.project_name.is_empty() {
        sch.project_id.clone()
    } else {
        sch.project_name.clone()
    };
    let task = format!("Onyx {reference}");
    let args = if install {
        let mut build = vec![
            "build".to_string(),
            "--project".to_string(),
            reference.clone(),
        ];
        if !sch.branch.is_empty() {
            build.push("--branch".to_string());
            build.push(sch.branch.clone());
        }
        let days = sch.next_ms.div_euclid(86_400_000);
        let (y, m, d) = schedule::civil_from_days(days);
        let rem = sch.next_ms.rem_euclid(86_400_000) / 1000;
        let (h, min) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32);
        let date = (sch.recurrence == Recurrence::Once).then_some((y, m, d));
        let timing = schedule::TaskTiming {
            rec: sch.recurrence,
            hour: h,
            minute: min,
            date,
            dow: schedule::weekday(days),
        };
        schedule::schtasks_create_args(&task, &exe, &build, &timing)
    } else {
        schedule::schtasks_delete_args(&task)
    };
    app.toasts.push(
        kit::ToastLevel::Success,
        if install {
            "Creating Windows task"
        } else {
            "Removing Windows task"
        },
    );
    std::thread::spawn(move || {
        let mut cmd = std::process::Command::new("schtasks");
        cmd.args(&args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let _ = cmd.status();
    });
}

fn stepper(ui: &mut egui::Ui, at: Pos2, label: &str, value: &mut u32, max: u32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    ui.painter().text(
        Pos2::new(at.x, at.y - px(14.0)),
        Align2::LEFT_TOP,
        label,
        FontId::new(px(10.5), theme::family_label()),
        theme::INK_MUTED,
    );
    let minus = Rect::from_min_size(Pos2::new(at.x, at.y), Vec2::splat(px(28.0)));
    let plus = Rect::from_min_size(
        Pos2::new(minus.right() + px(34.0), at.y),
        Vec2::splat(px(28.0)),
    );
    ui.painter().text(
        Pos2::new((minus.right() + plus.left()) / 2.0, at.y + px(14.0)),
        Align2::CENTER_CENTER,
        value.to_string(),
        FontId::new(px(13.0), theme::family_semibold()),
        theme::INK,
    );
    if Button::new("-", ButtonKind::Secondary)
        .show(ui, minus, s)
        .clicked()
    {
        *value = value.saturating_sub(1);
    }
    if Button::new("+", ButtonKind::Secondary)
        .show(ui, plus, s)
        .clicked()
        && *value < max
    {
        *value += 1;
    }
}

fn stepper_wide(ui: &mut egui::Ui, at: Pos2, label: &str, value: &mut i64, s: f32) {
    let px = |v: f32| theme::px(v, s);
    ui.painter().text(
        Pos2::new(at.x, at.y - px(14.0)),
        Align2::LEFT_TOP,
        label,
        FontId::new(px(10.5), theme::family_label()),
        theme::INK_MUTED,
    );
    let minus = Rect::from_min_size(Pos2::new(at.x, at.y), Vec2::splat(px(28.0)));
    let plus = Rect::from_min_size(
        Pos2::new(minus.right() + px(46.0), at.y),
        Vec2::splat(px(28.0)),
    );
    ui.painter().text(
        Pos2::new((minus.right() + plus.left()) / 2.0, at.y + px(14.0)),
        Align2::CENTER_CENTER,
        value.to_string(),
        FontId::new(px(13.0), theme::family_semibold()),
        theme::INK,
    );
    if Button::new("-", ButtonKind::Secondary)
        .show(ui, minus, s)
        .clicked()
    {
        *value -= 1;
    }
    if Button::new("+", ButtonKind::Secondary)
        .show(ui, plus, s)
        .clicked()
    {
        *value += 1;
    }
}
