use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::history;
use crate::model::{BuildRecord, BuildStatus, Project};
use crate::steamcmd::{self, BuildEvent, BuildHandle, BuildResult, PromptKind, Stage};
use crate::vdf::{self, BuildOptions};

const MAX_LINES: usize = 5000;

pub const STAGES: [(Stage, &str); 6] = [
    (Stage::Connecting, "Connecting"),
    (Stage::LoggingIn, "Logging in"),
    (Stage::Scanning, "Scanning"),
    (Stage::Uploading, "Uploading"),
    (Stage::Committing, "Committing"),
    (Stage::Finished, "Finished"),
];

pub const OPTIONS: [&str; 3] = ["Set live", "Verbose", "Validate"];

pub struct LogLine {
    pub text: String,
    pub error: bool,
}

pub struct BuildState {
    pub project_sel: usize,
    pub branch_sel: usize,
    pub description: String,
    pub preview_only: bool,
    pub backup: bool,
    pub verify: bool,
    pub options: [bool; 3],
    pub log: VecDeque<LogLine>,
    pub search: String,
    pub stage: Option<Stage>,
    pub progress: f32,
    pub outcome: Option<BuildResult>,
    pub running: bool,
    pub authorizing: bool,
    pub inited: bool,
    pub pending_prompt: Option<PromptKind>,
    pub prompt_input: String,
    pub finished_record: Option<BuildRecord>,
    pub preflight: super::preflight::PreflightState,
    pub review: super::review::ReviewState,
    pub pending_manifest_path: Option<PathBuf>,
    interactive: bool,
    run_project: Option<Project>,
    run_branch: String,
    log_path: Option<PathBuf>,
    started: Option<Instant>,
    finished_elapsed: Option<Duration>,
    handle: Option<BuildHandle>,
    tx: Sender<BuildEvent>,
    rx: Receiver<BuildEvent>,
}

impl BuildState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            project_sel: 0,
            branch_sel: 0,
            description: String::new(),
            preview_only: false,
            backup: false,
            verify: false,
            options: [false; 3],
            log: VecDeque::new(),
            search: String::new(),
            stage: None,
            progress: 0.0,
            outcome: None,
            running: false,
            authorizing: false,
            inited: false,
            pending_prompt: None,
            prompt_input: String::new(),
            finished_record: None,
            preflight: super::preflight::PreflightState::new(),
            review: super::review::ReviewState::new(),
            pending_manifest_path: None,
            interactive: true,
            run_project: None,
            run_branch: String::new(),
            log_path: None,
            started: None,
            finished_elapsed: None,
            handle: None,
            tx,
            rx,
        }
    }

    pub fn push_log(&mut self, text: impl Into<String>, error: bool) {
        self.log.push_back(LogLine {
            text: text.into(),
            error,
        });
        while self.log.len() > MAX_LINES {
            self.log.pop_front();
        }
    }

    pub fn log_text(&self) -> String {
        self.log
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn elapsed(&self) -> Duration {
        if self.running {
            self.started.map(|t| t.elapsed()).unwrap_or_default()
        } else {
            self.finished_elapsed.unwrap_or_default()
        }
    }

    pub fn filled_stages(&self) -> usize {
        match self.outcome {
            Some(BuildResult::Success { .. }) => STAGES.len(),
            _ => self
                .stage
                .and_then(|st| STAGES.iter().position(|(s, _)| *s == st))
                .map(|i| i + 1)
                .unwrap_or(0),
        }
    }

    pub fn stage_name(&self) -> &'static str {
        self.stage
            .and_then(|st| STAGES.iter().find(|(s, _)| *s == st).map(|(_, n)| *n))
            .unwrap_or("Idle")
    }

    pub fn drain(&mut self, ctx: &egui::Context) -> Option<BuildResult> {
        let mut finished = None;
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                BuildEvent::Log(s) => {
                    let lower = s.to_ascii_lowercase();
                    let error = lower.contains("error") || lower.contains("failed");
                    self.push_log(s, error);
                }
                BuildEvent::Stage(st) => self.stage = Some(st),
                BuildEvent::Progress(f) => self.progress = f.clamp(0.0, 1.0),
                BuildEvent::Prompt(kind) => {
                    let msg = match kind {
                        PromptKind::Password => "SteamCMD is asking for a password.",
                        PromptKind::SteamGuard => "SteamCMD is asking for a Steam Guard code.",
                    };
                    self.push_log(msg, false);
                    if self.interactive {
                        self.pending_prompt = Some(kind);
                    } else {
                        self.push_log(
                            "Non-interactive run needs a login. Run Authorize first.",
                            true,
                        );
                        self.cancel();
                    }
                }
                BuildEvent::Finished(res) => {
                    let was_auth = self.authorizing;
                    self.running = false;
                    self.authorizing = false;
                    self.pending_prompt = None;
                    self.prompt_input.clear();
                    self.finished_elapsed = self.started.map(|t| t.elapsed());
                    self.handle = None;
                    if !was_auth && let Some(project) = self.run_project.clone() {
                        let status = match &res {
                            BuildResult::Success { .. } => BuildStatus::Success,
                            BuildResult::Failure { .. } => BuildStatus::Failed,
                            BuildResult::Cancelled => BuildStatus::Cancelled,
                        };
                        let duration_ms = self
                            .finished_elapsed
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);
                        let log = self.log_path.clone().unwrap_or_default();
                        let mut record = history::make_record(
                            &project,
                            &self.run_branch,
                            &self.description,
                            status,
                            duration_ms,
                            log.as_path(),
                        );
                        if status == BuildStatus::Success {
                            record.manifest_path = self.pending_manifest_path.take();
                        } else {
                            self.pending_manifest_path = None;
                        }
                        self.finished_record = Some(record);
                    }
                    match &res {
                        BuildResult::Success { app_id } if app_id.is_empty() => {
                            self.push_log("Signed in. SteamCMD cached the login.", false)
                        }
                        BuildResult::Success { app_id } => {
                            self.push_log(format!("Build finished for app {app_id}."), false)
                        }
                        BuildResult::Failure { message } => {
                            self.push_log(format!("Build failed: {message}"), true)
                        }
                        BuildResult::Cancelled => self.push_log("Cancelled.", false),
                    }
                    self.outcome = Some(res.clone());
                    finished = Some(res);
                }
            }
        }
        if self.running {
            ctx.request_repaint();
        }
        finished
    }

    pub fn start(
        &mut self,
        project: &Project,
        branch: &str,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        let work = crate::config::build_dir().map_err(|e| e.to_string())?;
        let options = BuildOptions {
            preview: self.preview_only,
            local: None,
        };
        let paths = vdf::write_vdfs(project, &options, &work).map_err(|e| e.to_string())?;
        let app_build = paths
            .iter()
            .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("appbuild.vdf"))
            .cloned()
            .ok_or_else(|| "no appbuild.vdf was generated".to_string())?;
        let log_path = app_build.with_file_name("build.log");

        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        let opts = steamcmd::RunOptions {
            app_build_vdf: app_build,
            log_path: Some(log_path),
            interactive: true,
        };
        let handle = steamcmd::start_build(project, &opts, ctx, self.tx.clone())
            .map_err(|e| e.to_string())?;

        self.begin(handle);
        self.authorizing = false;
        self.run_project = Some(project.clone());
        self.run_branch = branch.to_string();
        self.push_log(
            if self.preview_only {
                "Starting preview build..."
            } else {
                "Starting build..."
            },
            false,
        );
        Ok(())
    }

    pub fn authorize(&mut self, project: &Project, ctx: &egui::Context) -> Result<(), String> {
        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        let handle =
            steamcmd::start_login(project, ctx, self.tx.clone()).map_err(|e| e.to_string())?;
        self.begin(handle);
        self.authorizing = true;
        self.push_log("Authorizing account. Answer any prompt below...", false);
        Ok(())
    }

    fn begin(&mut self, handle: BuildHandle) {
        self.log_path = Some(handle.log_path.clone());
        self.handle = Some(handle);
        self.finished_record = None;
        self.log.clear();
        self.outcome = None;
        self.stage = None;
        self.progress = 0.0;
        self.pending_prompt = None;
        self.prompt_input.clear();
        self.started = Some(Instant::now());
        self.finished_elapsed = None;
        self.running = true;
    }

    pub fn respond(&mut self) {
        let Some(kind) = self.pending_prompt else {
            return;
        };
        let value = std::mem::take(&mut self.prompt_input);
        if let Some(handle) = &self.handle {
            handle.send_input(&value);
        }
        let note = match kind {
            PromptKind::Password => "Password sent.",
            PromptKind::SteamGuard => "Steam Guard code sent.",
        };
        self.push_log(note, false);
        self.pending_prompt = None;
    }

    pub fn cancel(&mut self) {
        self.pending_prompt = None;
        self.prompt_input.clear();
        if let Some(handle) = &self.handle {
            steamcmd::cancel(handle);
            self.push_log("Cancelling...", false);
        }
    }
}
