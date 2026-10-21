use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, UiBuilder, Vec2};

use crate::app::App;
use crate::model::Project;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;
use crate::validate::{Fix, Issue, Severity};

pub struct PreflightState {
    open: bool,
    running: bool,
    branch: String,
    results: Vec<Issue>,
    tx: Sender<Vec<Issue>>,
    rx: Receiver<Vec<Issue>>,
}

impl PreflightState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            open: false,
            running: false,
            branch: String::new(),
            results: Vec::new(),
            tx,
            rx,
        }
    }

    pub fn request(&mut self, project: &Project, branch: &str, ctx: &egui::Context) {
        self.open = true;
        self.running = true;
        self.results.clear();
        self.branch = branch.to_string();
        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        let project = project.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let issues = crate::features::scanner::preflight(&project);
            let _ = tx.send(issues);
            ctx.request_repaint();
        });
    }

    fn drain(&mut self, ctx: &egui::Context) {
        while let Ok(results) = self.rx.try_recv() {
            self.results = results;
            self.running = false;
        }
        if self.running {
            ctx.request_repaint();
        }
    }

    fn blocking(&self) -> bool {
        self.results.iter().any(Issue::is_blocking)
    }
}

enum Act {
    None,
    Close,
    Proceed,
    Fix(usize),
    Ignore(usize),
}

fn dot_color(severity: Severity) -> egui::Color32 {
    match severity {
        Severity::Error => theme::DANGER,
        Severity::Warning => theme::WARNING,
        Severity::Info => theme::INK_MUTED,
    }
}

pub(crate) fn show(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) {
    app.build_ui.preflight.drain(ctx);
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id("preflight_modal");
    let mut open = app.build_ui.preflight.open;
    let results = app.build_ui.preflight.results.clone();
    let running = app.build_ui.preflight.running;
    let blocking = app.build_ui.preflight.blocking();
    let errors = results
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    let warnings = results
        .iter()
        .filter(|i| i.severity == Severity::Warning)
        .count();
    let mut act = Act::None;

    kit::modal_with(
        ui,
        id,
        &mut open,
        Vec2::new(px(560.0), px(466.0)),
        s,
        !running,
        |ui| {
            let d = ui.max_rect();
            let pad = px(24.0);
            let x = d.left() + pad;
            kit::label(
                &ui.painter().clone(),
                Pos2::new(x, d.top() + pad),
                "PRE-FLIGHT",
                s,
            );

            let summary = if running {
                "Checking files...".to_string()
            } else if results.is_empty() {
                "No problems found.".to_string()
            } else {
                format!("{errors} errors · {warnings} warnings")
            };
            ui.painter().text(
                Pos2::new(x, d.top() + px(50.0)),
                Align2::LEFT_TOP,
                summary,
                FontId::new(px(13.0), theme::family_medium()),
                theme::INK_DIM,
            );

            let list = Rect::from_min_max(
                Pos2::new(x, d.top() + px(80.0)),
                Pos2::new(d.right() - pad, d.bottom() - px(84.0)),
            );
            let mut lui = ui.new_child(UiBuilder::new().max_rect(list));
            egui::ScrollArea::vertical()
                .id_salt("preflight_list")
                .auto_shrink([false, false])
                .show(&mut lui, |ui| {
                    if let Some(a) = rows(ui, &results, s) {
                        act = a;
                    }
                });

            let by = d.bottom() - pad - px(40.0);
            let bw = px(150.0);
            if Button::new("CANCEL", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, by), Vec2::new(bw, px(40.0))),
                    s,
                )
                .clicked()
            {
                act = Act::Close;
            }
            let label = if results.iter().any(|i| i.severity != Severity::Info) {
                "BUILD ANYWAY"
            } else {
                "START BUILD"
            };
            let mut go = Button::new(label, ButtonKind::Primary)
                .icon(theme::icon::ROCKET)
                .enabled(!running && !blocking);
            if blocking {
                go = go.disabled_reason("Fix the errors above first");
            }
            if go
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - pad - bw, by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked()
                && !running
                && !blocking
            {
                act = Act::Proceed;
            }
            false
        },
    );

    app.build_ui.preflight.open = open;
    apply(app, act, ctx);
}

fn rows(ui: &mut egui::Ui, results: &[Issue], s: f32) -> Option<Act> {
    let px = |v: f32| theme::px(v, s);
    if results.is_empty() {
        return None;
    }
    let mut act = None;
    for (i, issue) in results.iter().enumerate() {
        let avail = ui.available_width();
        let has_fix = issue.fix.is_some();
        let can_ignore = issue.severity != Severity::Info;
        let btn_w = px(84.0);
        let gap = px(8.0);
        let mut controls = 0.0;
        if has_fix {
            controls += btn_w + gap;
        }
        if can_ignore {
            controls += btn_w + gap;
        }
        let text_w = (avail - px(26.0) - controls).max(px(80.0));
        let font = FontId::new(px(12.5), theme::family_medium());
        let galley = ui
            .painter()
            .layout(issue.message.clone(), font, theme::INK, text_w);
        let row_h = galley.size().y.max(px(22.0)) + px(16.0);
        let (rr, _) = ui.allocate_exact_size(Vec2::new(avail, row_h), Sense::hover());

        ui.painter().circle_filled(
            Pos2::new(rr.left() + px(8.0), rr.top() + px(13.0)),
            px(4.0),
            dot_color(issue.severity),
        );
        ui.painter().galley(
            Pos2::new(rr.left() + px(22.0), rr.top() + px(8.0)),
            galley,
            theme::INK,
        );

        let mut bx = rr.right() - btn_w;
        let by = rr.top() + (row_h - px(30.0)) * 0.5;
        if can_ignore
            && Button::new("IGNORE", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(bx, by), Vec2::new(btn_w, px(30.0))),
                    s,
                )
                .clicked()
        {
            act = Some(Act::Ignore(i));
        }
        if can_ignore {
            bx -= btn_w + gap;
        }
        if has_fix
            && Button::new("EXCLUDE", ButtonKind::Secondary)
                .icon(theme::icon::X)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(bx, by), Vec2::new(btn_w, px(30.0))),
                    s,
                )
                .clicked()
        {
            act = Some(Act::Fix(i));
        }
        ui.painter().hline(
            rr.left()..=rr.right(),
            rr.bottom(),
            egui::Stroke::new(1.0, theme::LINE),
        );
    }
    act
}

fn apply(app: &mut App, act: Act, ctx: &egui::Context) {
    match act {
        Act::None => {}
        Act::Close => app.build_ui.preflight.open = false,
        Act::Proceed => {
            app.build_ui.preflight.open = false;
            let sel = app.build_ui.project_sel;
            if let Some(project) = app.projects.get(sel).cloned() {
                let branch = app.build_ui.preflight.branch.clone();
                if !project.pipeline.is_empty() {
                    app.pipeline_ui.select(&project);
                    app.pipeline_ui.start(ctx, 0);
                    app.nav_request = Some(crate::app::Page::Pipelines);
                    app.toasts
                        .push(kit::ToastLevel::Success, "Running pipeline");
                } else {
                    super::review::begin(app, &project, &branch, ctx);
                }
            }
        }
        Act::Fix(i) => {
            let fix = app
                .build_ui
                .preflight
                .results
                .get(i)
                .and_then(|x| x.fix.clone());
            let Some(Fix::AddExclusion(pattern)) = fix else {
                return;
            };
            let sel = app.build_ui.project_sel;
            let Some(project) = app.projects.get_mut(sel) else {
                return;
            };
            if !project.ignore_patterns.contains(&pattern) {
                project.ignore_patterns.push(pattern);
            }
            let project = project.clone();
            persist_and_rescan(app, project, ctx);
            app.toasts.push(kit::ToastLevel::Success, "Exclusion added");
        }
        Act::Ignore(i) => {
            let Some(message) = app
                .build_ui
                .preflight
                .results
                .get(i)
                .map(|x| x.message.clone())
            else {
                return;
            };
            let sel = app.build_ui.project_sel;
            let Some(project) = app.projects.get_mut(sel) else {
                return;
            };
            if !project.ignored_findings.contains(&message) {
                project.ignored_findings.push(message);
            }
            let project = project.clone();
            persist_and_rescan(app, project, ctx);
            app.toasts.push(kit::ToastLevel::Warning, "Finding ignored");
        }
    }
}

fn persist_and_rescan(app: &mut App, project: Project, ctx: &egui::Context) {
    if let Err(e) = crate::config::save_project(&project) {
        eprintln!("save failed: {e}");
        app.toasts.push(kit::ToastLevel::Error, "Save failed");
        return;
    }
    app.projects = crate::config::list_projects().unwrap_or_default();
    let branch = app.build_ui.preflight.branch.clone();
    if let Some(fresh) = app.projects.iter().find(|p| p.id == project.id).cloned() {
        app.build_ui.preflight.request(&fresh, &branch, ctx);
    }
}
