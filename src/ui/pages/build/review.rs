mod view;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Pos2, Rect, Vec2};

use crate::app::App;
use crate::features::manifest::{self, DiffSummary, Manifest, Progress};
use crate::model::Project;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Filter {
    All,
    Added,
    Modified,
    Removed,
}

pub struct ReviewState {
    open: bool,
    running: bool,
    preview: bool,
    branch: String,
    project_id: String,
    progress: Arc<Progress>,
    result: Option<(Manifest, DiffSummary)>,
    tx: Sender<(Manifest, DiffSummary)>,
    rx: Receiver<(Manifest, DiffSummary)>,
    search: String,
    filter: Filter,
    sort_size: bool,
    save_on_ready: bool,
    skip: bool,
}

impl ReviewState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            open: false,
            running: false,
            preview: false,
            branch: String::new(),
            project_id: String::new(),
            progress: Arc::new(Progress::default()),
            result: None,
            tx,
            rx,
            search: String::new(),
            filter: Filter::All,
            sort_size: false,
            save_on_ready: false,
            skip: false,
        }
    }

    fn request(&mut self, project: &Project, previous: Option<PathBuf>, ctx: &egui::Context) {
        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        self.progress = Arc::new(Progress::default());
        self.result = None;
        self.running = true;
        self.search.clear();
        self.filter = Filter::All;
        self.sort_size = false;
        let project = project.clone();
        let progress = self.progress.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let manifest = manifest::build_manifest(&project, Some(&progress));
            let prev = previous.and_then(|p| manifest::load_manifest(&p).ok());
            let summary = manifest::diff(&manifest, prev.as_ref());
            let _ = tx.send((manifest, summary));
            ctx.request_repaint();
        });
    }
}

pub(crate) fn begin(app: &mut App, project: &Project, branch: &str, ctx: &egui::Context) {
    let settings = crate::config::load_settings().unwrap_or_default();
    let review = &mut app.build_ui.review;
    review.skip = settings.skip_review;
    review.preview = app.build_ui.preview_only;
    review.branch = branch.to_string();
    review.project_id = project.id.clone();
    let previous = app.history_ui.last_manifest(&project.id, branch);
    app.build_ui.review.request(project, previous, ctx);
    if app.build_ui.review.skip {
        app.build_ui.review.open = false;
        app.build_ui.review.save_on_ready = !app.build_ui.review.preview;
        start_build(app, project, branch, ctx);
    } else {
        app.build_ui.review.open = true;
        app.build_ui.review.save_on_ready = false;
    }
}

fn start_build(app: &mut App, project: &Project, branch: &str, ctx: &egui::Context) {
    match app.build_ui.start(project, branch, ctx) {
        Ok(()) => app.toasts.push(kit::ToastLevel::Success, "Build started"),
        Err(e) => {
            app.build_ui.push_log(format!("Could not start: {e}"), true);
            app.toasts
                .push(kit::ToastLevel::Error, "Could not start build");
        }
    }
}

pub(super) enum Act {
    None,
    Cancel,
    Build,
    Filter(Filter),
    ToggleSort,
    ToggleSkip,
}

pub(crate) fn show(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) {
    while let Ok(result) = app.build_ui.review.rx.try_recv() {
        app.build_ui.review.result = Some(result);
        app.build_ui.review.running = false;
    }
    if app.build_ui.review.running {
        ctx.request_repaint();
    }
    if app.build_ui.review.save_on_ready && !app.build_ui.review.running {
        let manifest = app.build_ui.review.result.as_ref().map(|(m, _)| m.clone());
        app.build_ui.review.save_on_ready = false;
        if let Some(m) = manifest {
            attach_manifest(app, &m);
        }
    }

    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id("review_modal");
    let mut open = app.build_ui.review.open;
    let running = app.build_ui.review.running;
    let (done, total) = {
        let p = &app.build_ui.review.progress;
        (
            p.done.load(Ordering::Relaxed),
            p.total.load(Ordering::Relaxed),
        )
    };
    let summary = app.build_ui.review.result.as_ref().map(|(_, d)| d.clone());
    let filter = app.build_ui.review.filter;
    let sort_size = app.build_ui.review.sort_size;
    let skip = app.build_ui.review.skip;
    let mut act = Act::None;
    let search = &mut app.build_ui.review.search;

    kit::modal(
        ui,
        id,
        &mut open,
        Vec2::new(px(700.0), px(580.0)),
        s,
        |ui| {
            let d = ui.max_rect();
            let pad = px(24.0);
            let x = d.left() + pad;
            kit::label(
                &ui.painter().clone(),
                Pos2::new(x, d.top() + pad),
                "REVIEW CHANGES",
                s,
            );

            if running || summary.is_none() {
                view::progress(ui, d, done, total, s);
            } else if let Some(summary) = &summary {
                view::summary_row(
                    ui,
                    Pos2::new(x, d.top() + px(52.0)),
                    d.width() - pad * 2.0,
                    summary,
                    s,
                );
                let cy = d.top() + px(108.0);
                view::controls(
                    ui,
                    x,
                    cy,
                    d.width() - pad * 2.0,
                    search,
                    filter,
                    sort_size,
                    &mut act,
                    s,
                );
                let list = Rect::from_min_max(
                    Pos2::new(x, cy + px(44.0)),
                    Pos2::new(d.right() - pad, d.bottom() - px(84.0)),
                );
                view::file_list(ui, list, summary, search, filter, sort_size, s);
            }

            let by = d.bottom() - pad - px(40.0);
            let mut skip_val = skip;
            let cb =
                Rect::from_min_size(Pos2::new(x, by + px(2.0)), Vec2::new(px(240.0), px(36.0)));
            if kit::checkbox(ui, cb, &mut skip_val, "Skip this review next time", s).clicked() {
                act = Act::ToggleSkip;
            }
            let bw = px(140.0);
            if Button::new("CANCEL", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - pad - bw * 2.0 - px(10.0), by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked()
            {
                act = Act::Cancel;
            }
            let go = Button::new("BUILD", ButtonKind::Primary)
                .icon(theme::icon::ROCKET)
                .enabled(!running)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - pad - bw, by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                );
            if go.clicked() && !running {
                act = Act::Build;
            }
            false
        },
    );

    app.build_ui.review.open = open;
    apply(app, act, ctx);
}

fn attach_manifest(app: &mut App, manifest: &Manifest) {
    match manifest::save_manifest(manifest) {
        Ok(path) => app.build_ui.pending_manifest_path = Some(path),
        Err(e) => eprintln!("could not save manifest: {e}"),
    }
}

fn apply(app: &mut App, act: Act, ctx: &egui::Context) {
    match act {
        Act::None => {}
        Act::Cancel => app.build_ui.review.open = false,
        Act::Filter(f) => app.build_ui.review.filter = f,
        Act::ToggleSort => app.build_ui.review.sort_size = !app.build_ui.review.sort_size,
        Act::ToggleSkip => {
            let value = !app.build_ui.review.skip;
            app.build_ui.review.skip = value;
            let mut settings = crate::config::load_settings().unwrap_or_default();
            settings.skip_review = value;
            if let Err(e) = crate::config::save_settings(&settings) {
                eprintln!("could not save settings: {e}");
            }
        }
        Act::Build => {
            let manifest = app.build_ui.review.result.as_ref().map(|(m, _)| m.clone());
            let preview = app.build_ui.review.preview;
            app.build_ui.review.open = false;
            if let Some(m) = &manifest
                && !preview
            {
                attach_manifest(app, m);
            }
            let branch = app.build_ui.review.branch.clone();
            let pid = app.build_ui.review.project_id.clone();
            if let Some(project) = app.projects.iter().find(|p| p.id == pid).cloned() {
                start_build(app, &project, &branch, ctx);
            }
        }
    }
}
