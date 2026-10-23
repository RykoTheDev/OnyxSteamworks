use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use crate::app::App;
use crate::features::pipeline::{self, PipelineEvent, PipelineHandle, PipelineOutcome};
use crate::features::queue::{
    Job, JobOptions, JobStatus, backoff_delay, classify_failure, should_retry,
};
use crate::features::schedule::{self, Schedule};
use crate::model::Project;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

pub struct QueueState {
    pub jobs: Vec<Job>,
    pub schedules: Vec<Schedule>,
    pub paused: bool,
    pub add_sel: usize,
    pub add_preview: bool,
    pub form: schedule::ScheduleForm,
    running: Option<usize>,
    handle: Option<PipelineHandle>,
    outcome_log: String,
    loaded: bool,
    tx: Sender<PipelineEvent>,
    rx: Receiver<PipelineEvent>,
}

impl QueueState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            jobs: Vec::new(),
            schedules: Vec::new(),
            paused: false,
            add_sel: 0,
            add_preview: false,
            form: schedule::ScheduleForm::default(),
            running: None,
            handle: None,
            outcome_log: String::new(),
            loaded: false,
            tx,
            rx,
        }
    }

    pub fn enqueue(
        &mut self,
        project: Project,
        branch: String,
        options: JobOptions,
        max_retries: u32,
    ) {
        self.jobs
            .push(Job::new(project, branch, options, max_retries));
    }

    pub fn tick(&mut self, projects: &[Project], ctx: &egui::Context) {
        if !self.loaded {
            self.schedules = schedule::load();
            self.loaded = true;
        }
        self.drain(ctx);
        self.fire_schedules(projects);

        let now = Instant::now();
        for job in &mut self.jobs {
            if job.status == JobStatus::Retrying && job.retry_at.is_some_and(|t| t <= now) {
                job.status = JobStatus::Queued;
                job.retry_at = None;
            }
        }
        if self.running.is_none()
            && !self.paused
            && let Some(i) = self.jobs.iter().position(|j| j.status == JobStatus::Queued)
        {
            self.start_job(i, ctx);
        }
        if self.running.is_some() || self.jobs.iter().any(|j| j.status == JobStatus::Retrying) {
            ctx.request_repaint();
        }
    }

    fn start_job(&mut self, i: usize, ctx: &egui::Context) {
        let (tx, rx) = channel();
        self.tx = tx;
        self.rx = rx;
        self.outcome_log.clear();
        let project = self.jobs[i].project.clone();
        self.jobs[i].status = JobStatus::Running;
        self.handle = Some(pipeline::start(&project, 0, ctx, self.tx.clone()));
        self.running = Some(i);
    }

    fn drain(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                PipelineEvent::StepLog(_, line, _) => {
                    self.outcome_log.push_str(&line);
                    self.outcome_log.push('\n');
                }
                PipelineEvent::Finished(outcome) => self.finish(outcome),
                _ => {}
            }
        }
        if self.running.is_some() {
            ctx.request_repaint();
        }
    }

    fn finish(&mut self, outcome: PipelineOutcome) {
        let Some(i) = self.running.take() else {
            return;
        };
        self.handle = None;
        let log = std::mem::take(&mut self.outcome_log);
        let job = &mut self.jobs[i];
        job.log = log.clone();
        match outcome {
            PipelineOutcome::Completed => job.status = JobStatus::Succeeded,
            PipelineOutcome::Cancelled => job.status = JobStatus::Cancelled,
            PipelineOutcome::Failed => {
                let (class, message) = classify_failure(&log, None);
                job.message = message;
                if should_retry(class, job.attempts, job.max_retries) {
                    job.attempts += 1;
                    job.status = JobStatus::Retrying;
                    job.retry_at = Some(Instant::now() + backoff_delay(job.attempts));
                } else {
                    job.status = JobStatus::Failed;
                }
            }
        }
    }

    fn fire_schedules(&mut self, projects: &[Project]) {
        let now = schedule::now_ms();
        let mut changed = false;
        let mut to_add = Vec::new();
        for sch in &mut self.schedules {
            if sch.enabled && sch.next_ms <= now {
                if let Some(p) = projects.iter().find(|p| p.id == sch.project_id) {
                    to_add.push((p.clone(), sch.branch.clone(), sch.preview));
                }
                match schedule::advance(sch.next_ms, sch.recurrence) {
                    Some(next) => sch.next_ms = next,
                    None => sch.enabled = false,
                }
                changed = true;
            }
        }
        let max = crate::features::queue::effective_max(
            crate::config::load_settings()
                .map(|s| s.queue_max_retries)
                .unwrap_or(0),
        );
        for (project, branch, preview) in to_add {
            self.enqueue(
                project,
                branch,
                JobOptions {
                    preview,
                    ..Default::default()
                },
                max,
            );
        }
        if changed {
            schedule::save(&self.schedules);
        }
    }

    pub fn move_job(&mut self, i: usize, up: bool) {
        let Some(j) = (if up {
            i.checked_sub(1)
        } else {
            i.checked_add(1).filter(|j| *j < self.jobs.len())
        }) else {
            return;
        };
        if self.running == Some(i) || self.running == Some(j) {
            return;
        }
        self.jobs.swap(i, j);
    }

    pub fn run_next(&mut self, i: usize) {
        let target = self.running.map(|r| r + 1).unwrap_or(0);
        if i > target && self.jobs[i].status == JobStatus::Queued {
            let job = self.jobs.remove(i);
            self.jobs.insert(target.min(self.jobs.len()), job);
        }
    }

    pub fn remove_job(&mut self, i: usize) {
        if Some(i) == self.running {
            if let Some(h) = &self.handle {
                h.cancel();
            }
            self.running = None;
            self.handle = None;
        }
        self.jobs.remove(i);
        if let Some(r) = self.running
            && r > i
        {
            self.running = Some(r - 1);
        }
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    header(app, ui, rect, s);
    let list = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + px(58.0)), rect.max);
    if app.queue.jobs.is_empty() {
        kit::empty_state(
            ui,
            list,
            theme::icon::STACK,
            "No queued runs",
            "Add the selected project to run it, or let a schedule enqueue one.",
            s,
        );
        return;
    }
    super::queue_cards::cards(app, ui, list, s);
}

fn header(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
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
    let dd = Rect::from_min_size(rect.min, Vec2::new(px(210.0), px(34.0)));
    if !refs.is_empty() {
        kit::dropdown(ui, dd, &refs, &mut app.queue.add_sel, s);
    }
    let prev = Rect::from_min_size(
        Pos2::new(dd.right() + px(12.0), rect.top() + px(5.0)),
        Vec2::new(px(44.0), px(24.0)),
    );
    let mut p = app.queue.add_preview;
    if kit::toggle(ui, prev, &mut p, s).changed() {
        app.queue.add_preview = p;
    }
    ui.painter().text(
        Pos2::new(prev.right() + px(8.0), prev.center().y),
        Align2::LEFT_CENTER,
        "Preview",
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK_DIM,
    );

    let add = Rect::from_min_size(
        Pos2::new(rect.right() - px(150.0), rect.top()),
        Vec2::new(px(150.0), px(34.0)),
    );
    let can_add = !app.projects.is_empty();
    if Button::new("Add to queue", ButtonKind::Primary)
        .icon(theme::icon::PLUS)
        .enabled(can_add)
        .show(ui, add, s)
        .clicked()
        && can_add
    {
        let project = app.projects[app.queue.add_sel.min(app.projects.len() - 1)].clone();
        let branch = project
            .branches
            .first()
            .map(|b| b.name.clone())
            .unwrap_or_default();
        let max = crate::features::queue::effective_max(
            crate::config::load_settings()
                .map(|c| c.queue_max_retries)
                .unwrap_or(0),
        );
        app.queue.enqueue(
            project,
            branch,
            JobOptions {
                preview: app.queue.add_preview,
                ..Default::default()
            },
            max,
        );
    }
    let pause = Rect::from_min_size(
        Pos2::new(add.left() - px(10.0) - px(120.0), rect.top()),
        Vec2::new(px(120.0), px(34.0)),
    );
    let label = if app.queue.paused { "Resume" } else { "Pause" };
    if Button::new(label, ButtonKind::Secondary)
        .show(ui, pause, s)
        .clicked()
    {
        app.queue.paused = !app.queue.paused;
    }
}
