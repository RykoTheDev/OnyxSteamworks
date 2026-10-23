use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use crate::features::pipeline::{self, PipelineEvent, PipelineHandle, PipelineOutcome, StepStatus};
use crate::features::presets::{self, Engine};
use crate::model::{Pipeline, PipelineStep, Project, StepKind};
use crate::ui::kit::ToastLevel;

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
    pub pipeline_sel: usize,
    pub draft: Project,
    pub expanded: Option<usize>,
    pub statuses: Vec<StepRunStatus>,
    pub logs: Vec<String>,
    pub awaiting: Option<(usize, String)>,
    pub outcome: Option<PipelineOutcome>,
    pub running: bool,
    pub run_saved: bool,
    running_pipeline: Option<String>,
    saved_pipelines: Vec<Pipeline>,
    handle: Option<PipelineHandle>,
    tx: Sender<PipelineEvent>,
    rx: Receiver<PipelineEvent>,
}

impl PipelineState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            project_sel: 0,
            pipeline_sel: 0,
            draft: Project::default(),
            expanded: None,
            statuses: Vec::new(),
            logs: Vec::new(),
            awaiting: None,
            outcome: None,
            running: false,
            run_saved: false,
            running_pipeline: None,
            saved_pipelines: Vec::new(),
            handle: None,
            tx,
            rx,
        }
    }

    pub fn sync(&mut self, projects: &[Project]) {
        if projects.is_empty() {
            return;
        }
        let sel = self.project_sel.min(projects.len() - 1);
        self.project_sel = sel;
        if self.draft.id != projects[sel].id {
            self.select(&projects[sel]);
        }
    }

    pub fn select(&mut self, project: &Project) {
        self.draft = project.clone();
        self.saved_pipelines = project.pipelines.clone();
        self.pipeline_sel = 0;
        self.reset_run_view();
    }

    fn reset_run_view(&mut self) {
        self.expanded = None;
        self.statuses.clear();
        self.logs.clear();
        self.awaiting = None;
        self.outcome = None;
    }

    pub fn current(&self) -> Option<&Pipeline> {
        self.draft.pipelines.get(self.pipeline_sel)
    }

    pub fn steps(&self) -> &[PipelineStep] {
        self.current().map(|p| p.steps.as_slice()).unwrap_or(&[])
    }

    pub fn steps_mut(&mut self) -> Option<&mut Vec<PipelineStep>> {
        self.draft
            .pipelines
            .get_mut(self.pipeline_sel)
            .map(|p| &mut p.steps)
    }

    pub fn shows_current_run(&self) -> bool {
        self.current().map(|p| &p.id) == self.running_pipeline.as_ref()
    }

    pub fn choose_pipeline(&mut self, index: usize) {
        if index != self.pipeline_sel && index < self.draft.pipelines.len() {
            self.pipeline_sel = index;
            if !self.running {
                self.reset_run_view();
            }
            self.expanded = None;
        }
    }

    pub fn dirty(&self) -> bool {
        self.draft.pipelines != self.saved_pipelines
    }

    pub fn mark_saved(&mut self) {
        self.saved_pipelines = self.draft.pipelines.clone();
    }

    pub fn revert(&mut self) {
        self.draft.pipelines = self.saved_pipelines.clone();
        self.pipeline_sel = self
            .pipeline_sel
            .min(self.draft.pipelines.len().saturating_sub(1));
        self.expanded = None;
    }

    pub fn add_pipeline(&mut self) {
        let engine =
            presets::detect(&crate::features::maintenance::resolve(&self.draft).content_root);
        let steps = engine.map(presets::preset_steps).unwrap_or_default();
        let name = if self.draft.pipelines.is_empty() {
            "Default".to_string()
        } else {
            let mut n = self.draft.pipelines.len() + 1;
            while self
                .draft
                .pipelines
                .iter()
                .any(|p| p.name == format!("Pipeline {n}"))
            {
                n += 1;
            }
            format!("Pipeline {n}")
        };
        self.draft.pipelines.push(Pipeline::new(&name, steps));
        self.pipeline_sel = self.draft.pipelines.len() - 1;
        if !self.running {
            self.reset_run_view();
        }
    }

    pub fn delete_pipeline(&mut self) {
        if self.pipeline_sel < self.draft.pipelines.len() {
            self.draft.pipelines.remove(self.pipeline_sel);
            self.pipeline_sel = self.pipeline_sel.saturating_sub(1);
            self.expanded = None;
        }
    }

    pub fn add_step(&mut self, kind: StepKind) {
        let engine =
            presets::detect(&crate::features::maintenance::resolve(&self.draft).content_root);
        let step = match (kind, engine) {
            (StepKind::BuildGame, Some(engine)) => presets::build_step(engine),
            _ => PipelineStep {
                id: uuid::Uuid::new_v4().to_string(),
                name: kind.label().to_string(),
                kind,
                enabled: true,
                platform: "Windows".to_string(),
                ..Default::default()
            },
        };
        if let Some(steps) = self.steps_mut() {
            steps.push(step);
            let last = steps.len() - 1;
            self.expanded = Some(last);
        }
    }

    pub fn apply_preset(&mut self, engine: Engine) {
        if let Some(steps) = self.steps_mut() {
            *steps = presets::preset_steps(engine);
        }
        self.expanded = None;
    }

    pub fn remove_step(&mut self, i: usize) {
        if let Some(steps) = self.steps_mut()
            && i < steps.len()
        {
            steps.remove(i);
        }
        self.expanded = None;
    }

    pub fn move_step(&mut self, i: usize, up: bool) {
        if let Some(steps) = self.steps_mut() {
            let j = if up {
                i.checked_sub(1)
            } else {
                i.checked_add(1).filter(|j| *j < steps.len())
            };
            if let Some(j) = j {
                steps.swap(i, j);
            }
        }
    }

    pub fn start(&mut self, ctx: &egui::Context, start_index: usize) {
        let Some(pipe) = self.current().cloned() else {
            return;
        };
        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        self.logs.clear();
        self.outcome = None;
        self.awaiting = None;
        let planned = pipeline::planned_indices(&pipe.steps, start_index);
        self.statuses = (0..pipe.steps.len())
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
            &pipe,
            start_index,
            ctx,
            self.tx.clone(),
        ));
        self.running_pipeline = Some(pipe.id);
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

    pub fn drain(&mut self, ctx: &egui::Context) -> Vec<(ToastLevel, String)> {
        let mut toasts = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                PipelineEvent::StepStarted(i) => self.set_status(i, StepRunStatus::Running),
                PipelineEvent::StepLog(i, line, _) => {
                    let name = self
                        .draft
                        .pipelines
                        .iter()
                        .find(|p| Some(&p.id) == self.running_pipeline.as_ref())
                        .and_then(|p| p.steps.get(i))
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| "step".to_string());
                    self.logs.push(format!("[{name}] {line}"));
                    if self.logs.len() > 2000 {
                        self.logs.remove(0);
                    }
                }
                PipelineEvent::StepFinished(i, status) => self.set_status(i, map_status(status)),
                PipelineEvent::AwaitConfirm(i, message) => self.awaiting = Some((i, message)),
                PipelineEvent::Notify(message) => {
                    if !message.trim().is_empty() {
                        toasts.push((ToastLevel::Success, message));
                    }
                }
                PipelineEvent::Finished(outcome) => {
                    self.running = false;
                    self.awaiting = None;
                    self.handle = None;
                    self.outcome = Some(outcome);
                    self.run_saved = true;
                    toasts.push(match outcome {
                        PipelineOutcome::Completed => {
                            (ToastLevel::Success, "Pipeline finished".to_string())
                        }
                        PipelineOutcome::Failed => {
                            (ToastLevel::Error, "Pipeline failed".to_string())
                        }
                        PipelineOutcome::Cancelled => {
                            (ToastLevel::Warning, "Pipeline cancelled".to_string())
                        }
                    });
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
