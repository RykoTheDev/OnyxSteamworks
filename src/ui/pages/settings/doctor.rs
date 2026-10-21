use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Shape, Stroke, Vec2};

use crate::app::App;
use crate::features::doctor::{self, CheckResult, Fix, Status};
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

const TITLES: [&str; 7] = [
    "SteamCMD",
    "Internet",
    "Disk space",
    "Write access",
    "Cached login",
    "Project config",
    "Build tools",
];

struct Row {
    title: String,
    result: Option<CheckResult>,
}

pub struct DoctorState {
    started: bool,
    username: String,
    rows: Vec<Row>,
    tx: Sender<(usize, CheckResult)>,
    rx: Receiver<(usize, CheckResult)>,
}

impl DoctorState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            started: false,
            username: String::new(),
            rows: Vec::new(),
            tx,
            rx,
        }
    }

    pub fn request(&mut self) {
        self.started = false;
    }

    fn start(&mut self, project: Option<crate::model::Project>, ctx: &egui::Context) {
        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        self.started = true;
        self.username = project
            .as_ref()
            .map(|p| p.steam_username.clone())
            .unwrap_or_default();
        self.rows = TITLES
            .iter()
            .map(|t| Row {
                title: t.to_string(),
                result: None,
            })
            .collect();
        let output = project.as_ref().map(|p| p.build_output_dir.clone());
        type Check = Box<dyn FnOnce() -> CheckResult + Send>;
        let p = project;
        let checks: Vec<Check> = vec![
            {
                let p = p.clone();
                Box::new(move || doctor::check_steamcmd(p))
            },
            Box::new(doctor::check_internet),
            {
                let o = output.clone();
                Box::new(move || doctor::check_disk(o))
            },
            {
                let o = output.clone();
                Box::new(move || doctor::check_write_access(o))
            },
            {
                let p = p.clone();
                Box::new(move || doctor::check_cached_login(p))
            },
            {
                let p = p.clone();
                Box::new(move || doctor::check_project(p))
            },
            { Box::new(move || doctor::check_tools(p)) },
        ];
        for (i, check) in checks.into_iter().enumerate() {
            let tx = self.tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let result = check();
                let _ = tx.send((i, result));
                ctx.request_repaint();
            });
        }
    }

    fn drain(&mut self, ctx: &egui::Context) {
        while let Ok((i, result)) = self.rx.try_recv() {
            if let Some(row) = self.rows.get_mut(i) {
                row.result = Some(result);
            }
        }
        if !crate::ui::anim::reduce_motion() && self.rows.iter().any(|r| r.result.is_none()) {
            ctx.request_repaint();
        }
    }

    fn results(&self) -> Vec<CheckResult> {
        self.rows.iter().filter_map(|r| r.result.clone()).collect()
    }
}

fn status_color(status: Status) -> egui::Color32 {
    match status {
        Status::Ok => theme::SUCCESS,
        Status::Warn => theme::WARNING,
        Status::Fail => theme::DANGER,
    }
}

fn status_glyph(status: Status) -> char {
    match status {
        Status::Ok => theme::icon::CHECK,
        Status::Warn => theme::icon::WARNING,
        Status::Fail => theme::icon::X,
    }
}

fn spinner(painter: &egui::Painter, center: Pos2, radius: f32, s: f32, time: f64) {
    let a = if crate::ui::anim::reduce_motion() {
        -std::f32::consts::FRAC_PI_2
    } else {
        (time * 4.0) as f32 % std::f32::consts::TAU
    };
    let steps = 16;
    let sweep = std::f32::consts::TAU * 0.72;
    let points: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let ang = a + sweep * (i as f32 / steps as f32);
            Pos2::new(center.x + radius * ang.cos(), center.y + radius * ang.sin())
        })
        .collect();
    painter.add(Shape::line(
        points,
        Stroke::new(theme::px(2.5, s), theme::INK_MUTED),
    ));
}

pub fn show(app: &mut App, ui: &mut egui::Ui, body: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    if !app.doctor_ui.started {
        let project = app.projects.get(app.build_ui.project_sel).cloned();
        app.doctor_ui.start(project, &ctx);
    }
    app.doctor_ui.drain(&ctx);

    let done = app
        .doctor_ui
        .rows
        .iter()
        .filter(|r| r.result.is_some())
        .count();
    let total = app.doctor_ui.rows.len().max(1);
    let all_done = done == total;
    let overall = doctor::overall(&app.doctor_ui.results());
    let ring_color = if all_done {
        status_color(overall)
    } else {
        theme::INK_MUTED
    };

    let ring_c = Pos2::new(body.left() + px(34.0), body.top() + px(34.0));
    kit::progress_ring(
        ui,
        ring_c,
        px(28.0),
        done as f32 / total as f32,
        ring_color,
        s,
    );
    ui.painter().text(
        ring_c,
        Align2::CENTER_CENTER,
        format!("{done}/{total}"),
        FontId::new(px(13.0), theme::family_semibold()),
        theme::INK,
    );
    let headline = if all_done {
        match overall {
            Status::Ok => "All checks passed",
            Status::Warn => "Some checks need attention",
            Status::Fail => "Problems found",
        }
    } else {
        "Running checks..."
    };
    ui.painter().text(
        Pos2::new(body.left() + px(80.0), body.top() + px(26.0)),
        Align2::LEFT_CENTER,
        headline,
        FontId::new(px(15.0), theme::family_semibold()),
        theme::INK,
    );

    if Button::new("RE-RUN", ButtonKind::Secondary)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(body.right() - px(310.0), body.top() + px(12.0)),
                Vec2::new(px(104.0), px(38.0)),
            ),
            s,
        )
        .clicked()
    {
        let project = app.projects.get(app.build_ui.project_sel).cloned();
        app.doctor_ui.start(project, &ctx);
    }
    if Button::new("COPY DIAGNOSTICS", ButtonKind::Primary)
        .icon(theme::icon::COPY)
        .enabled(all_done)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(body.right() - px(194.0), body.top() + px(12.0)),
                Vec2::new(px(194.0), px(38.0)),
            ),
            s,
        )
        .clicked()
        && all_done
    {
        let report = doctor::diagnostics_report(&app.doctor_ui.username, &app.doctor_ui.results());
        ui.ctx().copy_text(report);
        app.toasts
            .push(kit::ToastLevel::Success, "Diagnostics copied");
    }

    let list = Rect::from_min_max(Pos2::new(body.left(), body.top() + px(70.0)), body.max);
    rows(app, ui, list, s);
}

fn rows(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let row_h = px(58.0);
    let gap = px(10.0);
    let time = ui.input(|i| i.time);
    let mut fix: Option<Fix> = None;
    for (idx, row) in app.doctor_ui.rows.iter().enumerate() {
        let top = rect.top() + idx as f32 * (row_h + gap);
        if top + row_h > rect.bottom() {
            break;
        }
        let r = Rect::from_min_size(
            Pos2::new(rect.left(), top),
            Vec2::new(rect.width().min(px(640.0)), row_h),
        );
        kit::bordered(
            ui.painter(),
            r,
            theme::SURFACE_2,
            theme::LINE,
            px(theme::RADIUS_CARD),
        );
        let icon_c = Pos2::new(r.left() + px(26.0), r.center().y);
        match &row.result {
            None => spinner(ui.painter(), icon_c, px(9.0), s, time),
            Some(result) => {
                ui.painter().circle_filled(
                    icon_c,
                    px(11.0),
                    theme::with_alpha(status_color(result.status), 40),
                );
                ui.painter().text(
                    icon_c,
                    Align2::CENTER_CENTER,
                    status_glyph(result.status).to_string(),
                    theme::icon_font(px(12.0), false),
                    status_color(result.status),
                );
            }
        }
        ui.painter().text(
            Pos2::new(r.left() + px(50.0), r.top() + px(16.0)),
            Align2::LEFT_CENTER,
            &row.title,
            FontId::new(px(13.5), theme::family_semibold()),
            theme::INK,
        );
        let message = row
            .result
            .as_ref()
            .map(|x| x.message.as_str())
            .unwrap_or("Checking...");
        ui.painter().text(
            Pos2::new(r.left() + px(50.0), r.bottom() - px(16.0)),
            Align2::LEFT_CENTER,
            message,
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
        if let Some(result) = &row.result
            && let Some(action) = &result.fix
        {
            let btn = Rect::from_min_size(
                Pos2::new(r.right() - px(96.0), r.center().y - px(15.0)),
                Vec2::new(px(84.0), px(30.0)),
            );
            if Button::new("FIX", ButtonKind::Secondary)
                .show(ui, btn, s)
                .clicked()
            {
                fix = Some(action.clone());
            }
        }
    }
    if let Some(action) = fix {
        apply_fix(action);
        app.toasts.push(kit::ToastLevel::Success, "Opening fix");
    }
}

fn apply_fix(fix: Fix) {
    std::thread::spawn(move || {
        let _ = match fix {
            Fix::OpenUrl(url) => open::that(url),
            Fix::OpenFolder(path) => open::that(path),
        };
    });
}
