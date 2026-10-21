use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::history;
use crate::model::{BuildRecord, BuildStatus, Project};
use crate::steamcmd::{self, BuildEvent, BuildHandle, BuildResult, PromptKind, Stage};

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
    pub verify_ui: super::verify::VerifyState,
    pub pending_manifest_path: Option<PathBuf>,
    pub speed: super::speed::SpeedTracker,
    pub guard: super::guard::GuardState,
    pub(super) interactive: bool,
    pub(super) run_project: Option<Project>,
    pub(super) run_branch: String,
    pub(super) log_path: Option<PathBuf>,
    pub(super) started: Option<Instant>,
    pub(super) finished_elapsed: Option<Duration>,
    pub(super) handle: Option<BuildHandle>,
    pub(super) keepawake: Option<crate::keepawake::KeepAwake>,
    pub(super) tx: Sender<BuildEvent>,
    pub(super) rx: Receiver<BuildEvent>,
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
            verify_ui: super::verify::VerifyState::new(),
            pending_manifest_path: None,
            speed: super::speed::SpeedTracker::new(),
            guard: super::guard::GuardState::new(),
            interactive: true,
            run_project: None,
            run_branch: String::new(),
            log_path: None,
            started: None,
            finished_elapsed: None,
            handle: None,
            keepawake: None,
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
                BuildEvent::Progress(f) => {
                    self.progress = f.clamp(0.0, 1.0);
                    self.speed.record(self.progress, self.started);
                }
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
                    self.keepawake = None;
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
                        record.git_commit = crate::features::patchnotes::head_commit(&project);
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
