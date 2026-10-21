use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use crate::features::pipeline::{self, PipelineEvent, PipelineHandle, PipelineOutcome, StepStatus};
use crate::features::presets::Engine;
use crate::model::{PipelineStep, Project, StepKind};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum StepRunStatus {
    #[default]
    Idle,
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

pub struct PipelineState {
    pub project_sel: usize,
    pub draft: Project,
    pub expanded: Option<usize>,
    pub statuses: Vec<StepRunStatus>,
    pub logs: Vec<String>,
    pub awaiting: Option<(usize, String)>,
    pub outcome: Option<PipelineOutcome>,
    pub running: bool,
    saved_pipeline: Vec<PipelineStep>,
    handle: Option<PipelineHandle>,
    tx: Sender<PipelineEvent>,
    rx: Receiver<PipelineEvent>,
}

impl PipelineState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            project_sel: 0,
            draft: Project::default(),
            expanded: None,
            statuses: Vec::new(),
            logs: Vec::new(),
            awaiting: None,
            outcome: None,
            running: false,
            saved_pipeline: Vec::new(),
            handle: None,
            tx,
            rx,
        }
    }

    pub fn sync(&mut self, projects: &[Project], sel: usize) {
        if projects.is_empty() {
            return;
        }
        let sel = sel.min(projects.len() - 1);
        self.project_sel = sel;
        if self.draft.id != projects[sel].id {
            self.select(&projects[sel]);
        }
    }

    pub fn select(&mut self, project: &Project) {
        self.draft = project.clone();
        self.saved_pipeline = project.pipeline.clone();
        self.expanded = None;
        self.statuses.clear();
        self.logs.clear();
        self.awaiting = None;
        self.outcome = None;
    }

    pub fn dirty(&self) -> bool {
        self.draft.pipeline != self.saved_pipeline
    }

    pub fn mark_saved(&mut self) {
        self.saved_pipeline = self.draft.pipeline.clone();
    }

    pub fn segment_fill(&self, segments: usize) -> usize {
        let total = self.statuses.len();
        if total == 0 || segments == 0 {
            return 0;
        }
        let done = self
            .statuses
            .iter()
            .filter(|s| matches!(s, StepRunStatus::Success))
            .count();
        (((done as f32 / total as f32) * segments as f32).round() as usize).min(segments)
    }

    pub fn add_step(&mut self, kind: StepKind) {
        self.draft.pipeline.push(PipelineStep {
            id: uuid::Uuid::new_v4().to_string(),
            name: kind.label().to_string(),
            kind,
            enabled: true,
            ..Default::default()
        });
        self.expanded = Some(self.draft.pipeline.len() - 1);
    }

    pub fn apply_preset(&mut self, engine: Engine) {
        self.draft.pipeline = crate::features::presets::preset_steps(engine);
        self.expanded = None;
    }

    pub fn remove_step(&mut self, i: usize) {
        if i < self.draft.pipeline.len() {
            self.draft.pipeline.remove(i);
            self.expanded = None;
        }
    }

    pub fn move_step(&mut self, i: usize, up: bool) {
        let j = if up {
            i.checked_sub(1)
        } else {
            i.checked_add(1).filter(|j| *j < self.draft.pipeline.len())
        };
        if let Some(j) = j {
            self.draft.pipeline.swap(i, j);
        }
    }

    pub fn start(&mut self, ctx: &egui::Context, start_index: usize) {
        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        self.logs.clear();
        self.outcome = None;
        self.awaiting = None;
        let planned = pipeline::planned_indices(&self.draft.pipeline, start_index);
        self.statuses = (0..self.draft.pipeline.len())
            .map(|i| {
                if i < start_index {
                    StepRunStatus::Idle
                } else if planned.contains(&i) {
                    StepRunStatus::Pending
                } else {
                    StepRunStatus::Skipped
                }
            })
            .collect();
        self.handle = Some(pipeline::start(
            &self.draft,
            start_index,
            ctx,
            self.tx.clone(),
        ));
        self.running = true;
    }

    pub fn cancel(&mut self) {
        if let Some(handle) = &self.handle {
            handle.cancel();
        }
        self.awaiting = None;
    }

    pub fn confirm(&mut self) {
        if let Some(handle) = &self.handle {
            handle.confirm();
        }
        self.awaiting = None;
    }

    pub fn drain(&mut self, ctx: &egui::Context) -> Vec<String> {
        let mut toasts = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                PipelineEvent::StepStarted(i) => self.set_status(i, StepRunStatus::Running),
                PipelineEvent::StepLog(i, line, _) => {
                    let name = self
                        .draft
                        .pipeline
                        .get(i)
                        .map(|s| s.name.as_str())
                        .unwrap_or("step");
                    self.logs.push(format!("[{name}] {line}"));
                    if self.logs.len() > 2000 {
                        self.logs.remove(0);
                    }
                }
                PipelineEvent::StepFinished(i, status) => self.set_status(i, map_status(status)),
                PipelineEvent::AwaitConfirm(i, message) => self.awaiting = Some((i, message)),
                PipelineEvent::Notify(message) => {
                    if !message.trim().is_empty() {
                        toasts.push(message);
                    }
                }
                PipelineEvent::Finished(outcome) => {
                    self.running = false;
                    self.awaiting = None;
                    self.handle = None;
                    self.outcome = Some(outcome);
                }
            }
        }
        if self.running {
            ctx.request_repaint();
        }
        toasts
    }

    fn set_status(&mut self, i: usize, status: StepRunStatus) {
        if let Some(slot) = self.statuses.get_mut(i) {
            *slot = status;
        }
    }
}

fn map_status(status: StepStatus) -> StepRunStatus {
    match status {
        StepStatus::Success => StepRunStatus::Success,
        StepStatus::Failed => StepRunStatus::Failed,
        StepStatus::Skipped => StepRunStatus::Skipped,
    }
}
