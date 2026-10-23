use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, SystemTime};

use eframe::egui;

use crate::features::approvals::{self, Approval};
use crate::features::pipeline::{self, PipelineEvent, PipelineHandle, PipelineOutcome};
use crate::features::schedule::{self, Due, MissedPolicy, Recurrence, Schedule, TriggerKind};
use crate::features::{notify, triggers};
use crate::model::{Project, RunOutcome};
use crate::ui::kit::ToastLevel;

const POLL_SECS: f64 = 60.0;

pub struct ActiveRun {
    pub schedule_id: String,
    project_id: String,
    project_name: String,
    pipeline_name: String,
    notified: bool,
    pub title: String,
    pub step: String,
    pub awaiting: Option<String>,
    steps: Vec<String>,
    handle: PipelineHandle,
    rx: Receiver<PipelineEvent>,
}

pub struct ScheduleForm {
    pub project_sel: usize,
    pub pipeline_sel: usize,
    pub trigger: TriggerKind,
    pub watch_path: String,
    pub quiet_minutes: u32,
    pub tag_pattern: String,
    pub recurrence: Recurrence,
    pub hour: u32,
    pub minute: u32,
    pub weekday: u32,
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub missed: MissedPolicy,
    pub use_task: bool,
}

impl Default for ScheduleForm {
    fn default() -> Self {
        let (year, month, day, weekday) = schedule::local_today();
        Self {
            project_sel: 0,
            pipeline_sel: 0,
            trigger: TriggerKind::Time,
            watch_path: String::new(),
            quiet_minutes: 10,
            tag_pattern: "v*".to_string(),
            recurrence: Recurrence::Daily,
            hour: 9,
            minute: 0,
            weekday,
            year,
            month,
            day,
            missed: MissedPolicy::CatchUp,
            use_task: cfg!(windows),
        }
    }
}

enum TaskOp {
    Install,
    Remove,
    Toggle(bool),
}

pub struct SchedulerState {
    pub schedules: Vec<Schedule>,
    pub active: Option<ActiveRun>,
    pub form: ScheduleForm,
    pub run_saved: bool,
    pub approvals: Vec<Approval>,
    pub trigger_errors: HashMap<String, String>,
    stamp: Option<SystemTime>,
    approvals_stamp: Option<SystemTime>,
    seen_approvals: HashSet<String>,
    last_reload: f64,
    last_poll: f64,
    polling: bool,
    poll_tx: Sender<Vec<(String, Result<String, String>)>>,
    poll_rx: Receiver<Vec<(String, Result<String, String>)>>,
    task_tx: Sender<(String, TaskOp, Result<(), String>)>,
    task_rx: Receiver<(String, TaskOp, Result<(), String>)>,
}

impl SchedulerState {
    pub fn new() -> Self {
        let (task_tx, task_rx) = channel();
        let (poll_tx, poll_rx) = channel();
        let approvals = approvals::pending();
        Self {
            schedules: schedule::load(),
            active: None,
            form: ScheduleForm::default(),
            run_saved: false,
            seen_approvals: approvals.iter().map(|a| a.id.clone()).collect(),
            approvals,
            trigger_errors: HashMap::new(),
            stamp: schedule::file_stamp(),
            approvals_stamp: approvals::stamp(),
            last_reload: 0.0,
            last_poll: f64::NEG_INFINITY,
            polling: false,
            poll_tx,
            poll_rx,
            task_tx,
            task_rx,
        }
    }

    pub fn busy(&self) -> bool {
        self.active.is_some()
    }

    fn set(&mut self, schedules: Vec<Schedule>) {
        self.schedules = schedules;
        self.stamp = schedule::file_stamp();
    }

    pub fn tick(
        &mut self,
        projects: &[Project],
        pipeline_busy: bool,
        ctx: &egui::Context,
    ) -> Vec<(ToastLevel, String)> {
        let mut toasts = Vec::new();
        self.drain_active(&mut toasts, projects);
        self.drain_tasks(&mut toasts);

        let now_s = ctx.input(|i| i.time);
        if now_s - self.last_reload > 2.0 {
            self.last_reload = now_s;
            let stamp = schedule::file_stamp();
            if stamp != self.stamp {
                self.set(schedule::load());
            }
            self.reload_approvals(&mut toasts);
        }
        self.drain_polls(projects, pipeline_busy, ctx, &mut toasts);
        if !self.polling && now_s - self.last_poll >= POLL_SECS {
            self.last_poll = now_s;
            self.spawn_poll(projects, ctx);
        }

        if self.active.is_none() && !pipeline_busy {
            let now = schedule::now_ms();
            let next = self
                .schedules
                .iter()
                .map(|s| (s.id.clone(), schedule::due(s, now)))
                .find(|(_, due)| *due != Due::No);
            match next {
                Some((id, Due::Run { missed })) => {
                    if let Err(e) = self.start(&id, projects, ctx, true) {
                        toasts.push((ToastLevel::Error, e));
                    } else if missed {
                        toasts.push((
                            ToastLevel::Warning,
                            "Catching up a missed scheduled run".into(),
                        ));
                    }
                }
                Some((id, Due::Skip)) => {
                    let all = schedule::update(&id, |s| schedule::advance(s, now));
                    self.set(all);
                    toasts.push((ToastLevel::Warning, "Skipped a missed scheduled run".into()));
                }
                _ => {}
            }
        }

        if self.active.is_some() {
            ctx.request_repaint_after(Duration::from_millis(250));
        } else if !self.approvals.is_empty()
            || self
                .schedules
                .iter()
                .any(|s| s.enabled && !s.task_installed && !s.is_legacy())
        {
            ctx.request_repaint_after(Duration::from_secs(2));
        }
        toasts
    }

    fn reload_approvals(&mut self, toasts: &mut Vec<(ToastLevel, String)>) {
        let stamp = approvals::stamp();
        if stamp == self.approvals_stamp {
            return;
        }
        self.approvals_stamp = stamp;
        self.approvals = approvals::pending();
        for a in &self.approvals {
            if self.seen_approvals.insert(a.id.clone()) {
                toasts.push((
                    ToastLevel::Warning,
                    format!("{} is waiting for approval", a.pipeline_name),
                ));
            }
        }
    }

    pub fn decide(&mut self, id: &str, approve: bool) {
        approvals::decide(id, approve);
        self.approvals.retain(|a| a.id != id);
        self.approvals_stamp = approvals::stamp();
    }

    fn eligible_events(&self) -> Vec<Schedule> {
        self.schedules
            .iter()
            .filter(|s| s.enabled && s.is_event() && !s.task_installed && !s.is_legacy())
            .cloned()
            .collect()
    }

    fn spawn_poll(&mut self, projects: &[Project], ctx: &egui::Context) {
        let work: Vec<(Schedule, Project)> = self
            .eligible_events()
            .into_iter()
            .filter_map(|s| {
                let p = projects.iter().find(|p| p.id == s.project_id)?.clone();
                Some((s, p))
            })
            .collect();
        if work.is_empty() {
            return;
        }
        self.polling = true;
        let tx = self.poll_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let results = work
                .iter()
                .map(|(s, p)| (s.id.clone(), triggers::observe(s, p)))
                .collect();
            let _ = tx.send(results);
            ctx.request_repaint();
        });
    }

    fn drain_polls(
        &mut self,
        projects: &[Project],
        pipeline_busy: bool,
        ctx: &egui::Context,
        toasts: &mut Vec<(ToastLevel, String)>,
    ) {
        while let Ok(results) = self.poll_rx.try_recv() {
            self.polling = false;
            if self.active.is_some() || pipeline_busy {
                continue;
            }
            let now = schedule::now_ms();
            for (id, result) in results {
                let observed = match result {
                    Ok(v) => {
                        self.trigger_errors.remove(&id);
                        v
                    }
                    Err(e) => {
                        self.trigger_errors.insert(id, e);
                        continue;
                    }
                };
                let mut fire = false;
                let all = schedule::update(&id, |s| {
                    fire = triggers::step(s, &observed, now);
                    if fire {
                        triggers::fired(s, &observed);
                    }
                });
                self.set(all);
                if fire && self.active.is_none() {
                    match self.start(&id, projects, ctx, true) {
                        Ok(()) => toasts.push((
                            ToastLevel::Success,
                            "A trigger started a pipeline run".into(),
                        )),
                        Err(e) => toasts.push((ToastLevel::Error, e)),
                    }
                }
            }
        }
    }

    fn drain_active(&mut self, toasts: &mut Vec<(ToastLevel, String)>, projects: &[Project]) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        let mut finished = None;
        while let Ok(ev) = active.rx.try_recv() {
            match ev {
                PipelineEvent::StepStarted(i) => {
                    active.step = active.steps.get(i).cloned().unwrap_or_default();
                    active.awaiting = None;
                }
                PipelineEvent::AwaitConfirm(_, message) => {
                    if !active.notified {
                        active.notified = true;
                        let body =
                            Approval::new(&active.project_name, &active.pipeline_name, &message)
                                .body();
                        std::thread::spawn(move || {
                            for e in notify::broadcast("Approval needed", &body) {
                                eprintln!("{e}");
                            }
                        });
                        toasts.push((
                            ToastLevel::Warning,
                            format!("{} is waiting for approval", active.title),
                        ));
                    }
                    active.awaiting = Some(if message.trim().is_empty() {
                        "Waiting for approval".to_string()
                    } else {
                        message
                    });
                }
                PipelineEvent::Notify(message) if !message.trim().is_empty() => {
                    toasts.push((ToastLevel::Success, message));
                }
                PipelineEvent::Finished(outcome) => finished = Some(outcome),
                _ => {}
            }
        }
        let Some(outcome) = finished else {
            return;
        };
        let active = self.active.take().expect("active run");
        let result = match outcome {
            PipelineOutcome::Completed => RunOutcome::Completed,
            PipelineOutcome::Failed => RunOutcome::Failed,
            PipelineOutcome::Cancelled => RunOutcome::Cancelled,
        };
        let all = schedule::update(&active.schedule_id, |s| {
            s.last_finished_ms = schedule::now_ms();
            s.last_outcome = Some(result);
        });
        self.set(all);
        if let Some(s) = self
            .schedules
            .iter()
            .find(|s| s.id == active.schedule_id && s.trigger == TriggerKind::FolderChange)
            .cloned()
            && let Some(project) = projects.iter().find(|p| p.id == active.project_id).cloned()
        {
            std::thread::spawn(move || {
                schedule::update(&s.id, |sch| triggers::rebaseline_after_run(sch, &project));
            });
        }
        self.run_saved = true;
        toasts.push(match result {
            RunOutcome::Completed => (ToastLevel::Success, format!("{} finished", active.title)),
            RunOutcome::Failed => (ToastLevel::Error, format!("{} failed", active.title)),
            RunOutcome::Cancelled => (ToastLevel::Warning, format!("{} cancelled", active.title)),
        });
    }

    fn drain_tasks(&mut self, toasts: &mut Vec<(ToastLevel, String)>) {
        while let Ok((id, op, result)) = self.task_rx.try_recv() {
            match (op, result) {
                (TaskOp::Install, Ok(())) => {
                    let all = schedule::update(&id, |s| s.task_installed = true);
                    self.set(all);
                    toasts.push((ToastLevel::Success, "Registered with Task Scheduler".into()));
                }
                (TaskOp::Remove, Ok(())) => {
                    let all = schedule::update(&id, |s| s.task_installed = false);
                    self.set(all);
                }
                (TaskOp::Toggle(_), Ok(())) => {}
                (TaskOp::Install, Err(e)) => toasts.push((
                    ToastLevel::Error,
                    format!("Task Scheduler refused the task; it will run while Onyx is open. {e}"),
                )),
                (_, Err(e)) => toasts.push((ToastLevel::Error, format!("Task Scheduler: {e}"))),
            }
        }
    }

    fn spawn_task(&self, s: Schedule, op: TaskOp) {
        let tx = self.task_tx.clone();
        std::thread::spawn(move || {
            let result = match op {
                TaskOp::Install => schedule::install_task(&s),
                TaskOp::Remove => schedule::remove_task(&s),
                TaskOp::Toggle(on) => schedule::set_task_enabled(&s, on),
            };
            let _ = tx.send((s.id.clone(), op, result));
        });
    }

    fn start(
        &mut self,
        id: &str,
        projects: &[Project],
        ctx: &egui::Context,
        scheduled: bool,
    ) -> Result<(), String> {
        let now = schedule::now_ms();
        let Some(s) = self.schedules.iter().find(|s| s.id == id).cloned() else {
            return Err("That schedule no longer exists".into());
        };
        let found = projects
            .iter()
            .find(|p| p.id == s.project_id)
            .and_then(|p| {
                p.pipelines
                    .iter()
                    .find(|x| x.id == s.pipeline_id)
                    .map(|x| (p, x))
            });
        let all = schedule::update(id, |sch| {
            sch.last_started_ms = now;
            if scheduled {
                schedule::advance(sch, now);
            }
        });
        self.set(all);
        let Some((project, pipe)) = found else {
            return Err(format!(
                "Can't run the {} schedule: its project or pipeline was deleted",
                s.pipeline_name
            ));
        };
        if pipe.steps.is_empty() {
            return Err(format!("The {} pipeline has no steps", pipe.name));
        }
        let (tx, rx) = channel();
        let handle = pipeline::start(project, pipe, 0, ctx, tx);
        self.active = Some(ActiveRun {
            schedule_id: s.id.clone(),
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            pipeline_name: pipe.name.clone(),
            notified: false,
            title: format!("{} ({})", pipe.name, project.name),
            step: String::new(),
            awaiting: None,
            steps: pipe.steps.iter().map(|x| x.name.clone()).collect(),
            handle,
            rx,
        });
        Ok(())
    }

    pub fn run_now(
        &mut self,
        id: &str,
        projects: &[Project],
        ctx: &egui::Context,
    ) -> Result<(), String> {
        if self.active.is_some() {
            return Err("A scheduled run is already in progress".into());
        }
        self.start(id, projects, ctx, false)
    }

    pub fn approve(&mut self) {
        if let Some(active) = self.active.as_mut() {
            active.handle.confirm();
            active.awaiting = None;
        }
    }

    pub fn cancel(&mut self) {
        if let Some(active) = &self.active {
            active.handle.cancel();
        }
    }

    pub fn add(&mut self, projects: &[Project]) -> Result<String, String> {
        let f = &self.form;
        let project = projects.get(f.project_sel).ok_or("Pick a project")?;
        let pipe = project
            .pipelines
            .get(f.pipeline_sel)
            .ok_or("Create a pipeline in Automation > Pipelines first")?;
        let mut s = Schedule {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: project.id.clone(),
            project_name: if project.name.is_empty() {
                "Untitled".into()
            } else {
                project.name.clone()
            },
            pipeline_id: pipe.id.clone(),
            pipeline_name: pipe.name.clone(),
            trigger: f.trigger,
            watch_path: f.watch_path.trim().to_string(),
            quiet_minutes: f.quiet_minutes.max(1),
            tag_pattern: f.tag_pattern.trim().to_string(),
            recurrence: f.recurrence,
            hour: f.hour,
            minute: f.minute,
            weekday: f.weekday,
            year: f.year,
            month: f.month,
            day: f.day,
            missed: f.missed,
            enabled: true,
            ..Default::default()
        };
        let message = if s.is_event() {
            "Watching. The first check sets the starting point.".to_string()
        } else {
            s.next_ms = schedule::next_run(&s, schedule::now_ms())
                .ok_or("That date and time has already passed")?;
            format!("Scheduled. Next run {}", schedule::fmt_local(s.next_ms))
        };
        let use_task = f.use_task && cfg!(windows);
        self.set(schedule::insert(s.clone()));
        if use_task {
            self.spawn_task(s, TaskOp::Install);
        }
        self.last_poll = f64::NEG_INFINITY;
        Ok(message)
    }

    pub fn remove(&mut self, id: &str) {
        if let Some(s) = self.schedules.iter().find(|s| s.id == id).cloned()
            && s.task_installed
        {
            self.spawn_task(s, TaskOp::Remove);
        }
        self.set(schedule::remove(id));
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        let now = schedule::now_ms();
        let all = schedule::update(id, |s| {
            s.enabled = enabled;
            if enabled && (s.next_ms == 0 || s.next_ms < now) {
                schedule::advance(s, now);
            }
        });
        self.set(all);
        if let Some(s) = self.schedules.iter().find(|s| s.id == id).cloned()
            && s.task_installed
        {
            self.spawn_task(s, TaskOp::Toggle(enabled));
        }
    }

    pub fn set_task(&mut self, id: &str, install: bool) {
        if let Some(s) = self.schedules.iter().find(|s| s.id == id).cloned() {
            self.spawn_task(
                s,
                if install {
                    TaskOp::Install
                } else {
                    TaskOp::Remove
                },
            );
        }
    }
}
