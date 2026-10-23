mod view;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Pos2, Rect, Vec2};

use crate::app::App;
use crate::features::manifest;
use crate::features::verify::{self, Progress, VerifyReport};
use crate::model::{BuildRecord, Project};
use crate::steamcmd::{self, BuildEvent, BuildHandle, BuildResult, VerifyOptions};
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

pub(super) enum Phase {
    Idle,
    Downloading,
    Hashing,
    Done(VerifyReport),
    Access(String),
    Failed(String),
}

enum Outcome {
    Report(VerifyReport),
    NoManifest,
    Error(String),
}

pub struct VerifyState {
    phase: Phase,
    open: bool,
    running: bool,
    keep_temp: bool,
    temp: Option<PathBuf>,
    manifest_path: Option<PathBuf>,
    log: VecDeque<String>,
    progress: Arc<Progress>,
    handle: Option<BuildHandle>,
    steam_tx: Sender<BuildEvent>,
    steam_rx: Receiver<BuildEvent>,
    out_tx: Sender<Outcome>,
    out_rx: Receiver<Outcome>,
}

impl VerifyState {
    pub fn new() -> Self {
        let (steam_tx, steam_rx) = channel();
        let (out_tx, out_rx) = channel();
        Self {
            phase: Phase::Idle,
            open: false,
            running: false,
            keep_temp: false,
            temp: None,
            manifest_path: None,
            log: VecDeque::new(),
            progress: Arc::new(Progress::default()),
            handle: None,
            steam_tx,
            steam_rx,
            out_tx,
            out_rx,
        }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    fn log_text(&self) -> String {
        self.log.iter().cloned().collect::<Vec<_>>().join("\n")
    }

    fn cleanup(&mut self) {
        if !self.keep_temp
            && let Some(dir) = &self.temp
        {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

pub(crate) fn begin(app: &mut App, project: &Project, record: BuildRecord, ctx: &egui::Context) {
    let keep = crate::config::load_settings()
        .map(|s| s.keep_verify_temp)
        .unwrap_or(false);
    let base = crate::config::data_dir().unwrap_or_else(|_| std::env::temp_dir());
    let temp = base.join("verify").join(uuid::Uuid::new_v4().to_string());
    let _ = std::fs::create_dir_all(&temp);

    let (steam_tx, steam_rx) = channel();
    {
        let v = &mut app.build_ui.verify_ui;
        v.phase = Phase::Downloading;
        v.open = true;
        v.running = true;
        v.keep_temp = keep;
        v.temp = Some(temp.clone());
        v.manifest_path = record.manifest_path.clone();
        v.log.clear();
        v.handle = None;
        v.steam_tx = steam_tx;
        v.steam_rx = steam_rx;
    }
    let opts = VerifyOptions {
        app_id: project.app_id.clone(),
        install_dir: temp,
        branch: record.branch.clone(),
        beta_password: String::new(),
        log_path: None,
    };
    let tx = app.build_ui.verify_ui.steam_tx.clone();
    match steamcmd::start_verify(project, &opts, ctx, tx) {
        Ok(handle) => app.build_ui.verify_ui.handle = Some(handle),
        Err(e) => {
            app.build_ui.verify_ui.phase = Phase::Failed(format!("Could not start verify: {e}"));
            app.build_ui.verify_ui.running = false;
        }
    }
}

pub(crate) fn drain(app: &mut App, ctx: &egui::Context) {
    while let Ok(event) = app.build_ui.verify_ui.steam_rx.try_recv() {
        match event {
            BuildEvent::Log(line) => {
                let v = &mut app.build_ui.verify_ui;
                v.log.push_back(line);
                while v.log.len() > 200 {
                    v.log.pop_front();
                }
            }
            BuildEvent::Prompt(_) => {
                if let Some(h) = &app.build_ui.verify_ui.handle {
                    steamcmd::cancel(h);
                }
                app.build_ui.verify_ui.phase = Phase::Access(
                    "The build account needs to sign in first. Authorize the account, then verify."
                        .to_string(),
                );
                app.build_ui.verify_ui.running = false;
                app.build_ui.verify_ui.cleanup();
            }
            BuildEvent::Finished(result) => on_finished(app, result, ctx),
            _ => {}
        }
    }
    while let Ok(outcome) = app.build_ui.verify_ui.out_rx.try_recv() {
        let v = &mut app.build_ui.verify_ui;
        v.phase = match outcome {
            Outcome::Report(report) => Phase::Done(report),
            Outcome::NoManifest => Phase::Failed(
                "No manifest was saved for this build, so there is nothing to compare against."
                    .to_string(),
            ),
            Outcome::Error(message) => Phase::Failed(message),
        };
        v.running = false;
        v.cleanup();
    }
    if app.build_ui.verify_ui.running {
        ctx.request_repaint();
    }
}

fn on_finished(app: &mut App, result: BuildResult, ctx: &egui::Context) {
    match result {
        BuildResult::Success { .. } => {
            let (out_tx, out_rx) = channel();
            let v = &mut app.build_ui.verify_ui;
            v.phase = Phase::Hashing;
            v.progress = Arc::new(Progress::default());
            v.out_tx = out_tx;
            v.out_rx = out_rx;
            let temp = v.temp.clone();
            let manifest_path = v.manifest_path.clone();
            let progress = v.progress.clone();
            let tx = v.out_tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let outcome = match (temp, manifest_path) {
                    (Some(dir), Some(path)) => {
                        let downloaded = verify::hash_dir(&dir, Some(&progress));
                        match manifest::load_manifest(&path) {
                            Ok(manifest) => {
                                Outcome::Report(verify::compare(&manifest, &downloaded))
                            }
                            Err(e) => {
                                Outcome::Error(format!("Could not read the saved manifest: {e}"))
                            }
                        }
                    }
                    (_, None) => Outcome::NoManifest,
                    (None, _) => Outcome::Error("No temp folder was available.".to_string()),
                };
                let _ = tx.send(outcome);
                ctx.request_repaint();
            });
        }
        BuildResult::Failure { message } => {
            let v = &mut app.build_ui.verify_ui;
            v.phase = match verify::access_issue(&v.log_text()) {
                Some(msg) => Phase::Access(msg.to_string()),
                None => Phase::Failed(format!("Verify download failed: {message}")),
            };
            v.running = false;
            v.cleanup();
        }
        BuildResult::Cancelled => {
            let v = &mut app.build_ui.verify_ui;
            v.phase = Phase::Failed("Verify was cancelled.".to_string());
            v.running = false;
            v.cleanup();
        }
    }
}

pub(crate) fn show(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) {
    drain(app, ctx);
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id("verify_modal");
    let mut open = app.build_ui.verify_ui.open;
    let running = app.build_ui.verify_ui.running;
    let (done, total) = {
        let p = &app.build_ui.verify_ui.progress;
        (
            p.done.load(Ordering::Relaxed),
            p.total.load(Ordering::Relaxed),
        )
    };
    let mut cancel = false;
    let mut copy = false;
    let mut close = false;

    kit::modal_with(
        ui,
        id,
        &mut open,
        Vec2::new(px(560.0), px(440.0)),
        s,
        !running,
        |ui| {
            let d = ui.max_rect();
            let pad = px(24.0);
            let x = d.left() + pad;
            kit::label(
                &ui.painter().clone(),
                Pos2::new(x, d.top() + pad),
                "Verify build",
                s,
            );
            view::body(ui, &app.build_ui.verify_ui.phase, done, total, s);

            let by = d.bottom() - pad - px(40.0);
            let bw = px(150.0);
            if running {
                if Button::new("Cancel", ButtonKind::Secondary)
                    .show(
                        ui,
                        Rect::from_min_size(Pos2::new(x, by), Vec2::new(bw, px(40.0))),
                        s,
                    )
                    .clicked()
                {
                    cancel = true;
                }
            } else {
                if matches!(app.build_ui.verify_ui.phase, Phase::Done(_))
                    && Button::new("Copy report", ButtonKind::Secondary)
                        .icon(theme::icon::COPY)
                        .show(
                            ui,
                            Rect::from_min_size(Pos2::new(x, by), Vec2::new(px(170.0), px(40.0))),
                            s,
                        )
                        .clicked()
                {
                    copy = true;
                }
                if Button::new("Close", ButtonKind::Primary)
                    .show(
                        ui,
                        Rect::from_min_size(
                            Pos2::new(d.right() - pad - bw, by),
                            Vec2::new(bw, px(40.0)),
                        ),
                        s,
                    )
                    .clicked()
                {
                    close = true;
                }
            }
            false
        },
    );

    if cancel && let Some(h) = &app.build_ui.verify_ui.handle {
        steamcmd::cancel(h);
    }
    if copy && let Phase::Done(report) = &app.build_ui.verify_ui.phase {
        ui.ctx().copy_text(report.text());
        app.toasts.push(kit::ToastLevel::Success, "Report copied");
    }
    app.build_ui.verify_ui.open = if close { false } else { open };
}
