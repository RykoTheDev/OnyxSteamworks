#![allow(dead_code)]

mod actions;
mod changes;
mod exec;

pub use exec::split_args;

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use eframe::egui;

use crate::model::{
    Pipeline, PipelineRun, PipelineStep, Project, RunIf, RunOutcome, StepOutcome, StepRecord,
};

const LOG_KEEP: usize = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineOutcome {
    Completed,
    Failed,
    Cancelled,
}

pub enum PipelineEvent {
    StepStarted(usize),
    StepLog(usize, String, bool),
    StepFinished(usize, StepStatus),
    AwaitConfirm(usize, String),
    Notify(String),
    Finished(PipelineOutcome),
}

#[derive(Default)]
pub(super) struct Control {
    pub(super) cancelled: AtomicBool,
    pub(super) confirm_flag: Mutex<bool>,
    pub(super) confirm_cv: Condvar,
    pub(super) child_pid: Mutex<Option<u32>>,
}

pub struct PipelineHandle {
    control: Arc<Control>,
}

impl PipelineHandle {
    pub fn cancel(&self) {
        self.control.cancelled.store(true, Ordering::SeqCst);
        if let Ok(mut flag) = self.control.confirm_flag.lock() {
            *flag = true;
        }
        self.control.confirm_cv.notify_all();
        let pid = self.control.child_pid.lock().ok().and_then(|g| *g);
        if let Some(pid) = pid {
            kill_tree(pid);
        }
    }

    pub fn confirm(&self) {
        if let Ok(mut flag) = self.control.confirm_flag.lock() {
            *flag = true;
        }
        self.control.confirm_cv.notify_all();
    }
}

pub fn planned_indices(steps: &[PipelineStep], start_index: usize) -> Vec<usize> {
    steps
        .iter()
        .enumerate()
        .filter(|(i, step)| *i >= start_index && step.enabled)
        .map(|(i, _)| i)
        .collect()
}

pub fn start(
    project: &Project,
    pipeline: &Pipeline,
    start_index: usize,
    ctx: &egui::Context,
    tx: Sender<PipelineEvent>,
) -> PipelineHandle {
    let control = Arc::new(Control::default());
    let project = project.clone();
    let pipeline = pipeline.clone();
    let ctx = ctx.clone();
    let worker = control.clone();
    std::thread::spawn(move || {
        let (itx, irx) = channel();
        let recorder = {
            let run = new_run(&project, &pipeline);
            let ctx = ctx.clone();
            std::thread::spawn(move || record(run, irx, tx, &ctx, &mut save_run))
        };
        let outcome = run_steps(&project, &pipeline.steps, start_index, &worker, &ctx, &itx);
        let _ = itx.send(PipelineEvent::Finished(outcome));
        drop(itx);
        let _ = recorder.join();
        ctx.request_repaint();
    });
    PipelineHandle { control }
}

fn new_run(project: &Project, pipeline: &Pipeline) -> PipelineRun {
    PipelineRun {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        pipeline_id: pipeline.id.clone(),
        pipeline_name: pipeline.name.clone(),
        started_ms: now_ms(),
        steps: pipeline
            .steps
            .iter()
            .map(|s| StepRecord {
                name: s.name.clone(),
                kind: s.kind,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

fn record(
    mut run: PipelineRun,
    rx: Receiver<PipelineEvent>,
    tx: Sender<PipelineEvent>,
    ctx: &egui::Context,
    save: &mut dyn FnMut(&PipelineRun),
) {
    let begun = Instant::now();
    let mut started: Vec<Option<Instant>> = vec![None; run.steps.len()];
    while let Ok(event) = rx.recv() {
        match &event {
            PipelineEvent::StepStarted(i) => {
                if let Some(slot) = started.get_mut(*i) {
                    *slot = Some(Instant::now());
                }
            }
            PipelineEvent::StepLog(i, line, _) => {
                if let Some(step) = run.steps.get_mut(*i) {
                    step.log.push(line.clone());
                    if step.log.len() > LOG_KEEP {
                        step.log.remove(0);
                    }
                }
            }
            PipelineEvent::StepFinished(i, status) => {
                if let Some(step) = run.steps.get_mut(*i) {
                    step.outcome = match status {
                        StepStatus::Success => StepOutcome::Success,
                        StepStatus::Failed => StepOutcome::Failed,
                        StepStatus::Skipped => StepOutcome::Skipped,
                    };
                    step.duration_ms = started
                        .get(*i)
                        .copied()
                        .flatten()
                        .map(|t| t.elapsed().as_millis() as u64)
                        .unwrap_or(0);
                }
            }
            PipelineEvent::Finished(outcome) => {
                run.outcome = match outcome {
                    PipelineOutcome::Completed => RunOutcome::Completed,
                    PipelineOutcome::Failed => RunOutcome::Failed,
                    PipelineOutcome::Cancelled => RunOutcome::Cancelled,
                };
                run.duration_ms = begun.elapsed().as_millis() as u64;
                save(&run);
            }
            _ => {}
        }
        let _ = tx.send(event);
        ctx.request_repaint();
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn save_run(run: &PipelineRun) {
    let saved = crate::history::connect().and_then(|conn| crate::history::insert_run(&conn, run));
    if let Err(e) = saved {
        eprintln!("could not save pipeline run: {e}");
    }
}

fn run_steps(
    project: &Project,
    steps: &[PipelineStep],
    start_index: usize,
    control: &Control,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> PipelineOutcome {
    let mut failed = false;
    for (i, step) in steps.iter().enumerate() {
        if i < start_index {
            continue;
        }
        if control.cancelled.load(Ordering::SeqCst) {
            return PipelineOutcome::Cancelled;
        }
        if !wanted(step, failed) {
            let _ = tx.send(PipelineEvent::StepFinished(i, StepStatus::Skipped));
            continue;
        }
        let _ = tx.send(PipelineEvent::StepStarted(i));
        ctx.request_repaint();
        if step.run_if == RunIf::FilesChanged {
            match changes::files_changed(project) {
                Ok(0) => {
                    exec::emit(
                        tx,
                        ctx,
                        i,
                        "No files changed since the last upload; skipping.",
                        false,
                    );
                    let _ = tx.send(PipelineEvent::StepFinished(i, StepStatus::Skipped));
                    continue;
                }
                Ok(usize::MAX) => exec::emit(
                    tx,
                    ctx,
                    i,
                    "No earlier upload to compare with; running.",
                    false,
                ),
                Ok(n) => exec::emit(
                    tx,
                    ctx,
                    i,
                    format!("{n} file(s) changed since the last upload."),
                    false,
                ),
                Err(e) => exec::emit(
                    tx,
                    ctx,
                    i,
                    format!("Could not compare files ({e}); running anyway."),
                    true,
                ),
            }
        }
        let status = exec::run_step(project, i, step, control, ctx, tx);
        let _ = tx.send(PipelineEvent::StepFinished(i, status));
        ctx.request_repaint();
        if control.cancelled.load(Ordering::SeqCst) {
            return PipelineOutcome::Cancelled;
        }
        if status == StepStatus::Failed && !step.continue_on_failure {
            failed = true;
        }
    }
    if control.cancelled.load(Ordering::SeqCst) {
        PipelineOutcome::Cancelled
    } else if failed {
        PipelineOutcome::Failed
    } else {
        PipelineOutcome::Completed
    }
}

pub fn wanted(step: &PipelineStep, failed: bool) -> bool {
    step.enabled
        && match step.run_if {
            RunIf::PreviousFailed => failed,
            _ => !failed,
        }
}

#[cfg(windows)]
fn kill_tree(pid: u32) {
    use std::os::windows::process::CommandExt;
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(0x0800_0000)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(windows))]
fn kill_tree(pid: u32) {
    let _ = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PipelineStep;

    fn step(name: &str, enabled: bool) -> PipelineStep {
        PipelineStep {
            name: name.to_string(),
            enabled,
            ..Default::default()
        }
    }

    #[test]
    fn recorder_captures_step_results_then_forwards_events() {
        let pipeline = Pipeline::new("Nightly", vec![step("build", true), step("upload", true)]);
        let project = Project {
            id: "p1".into(),
            name: "Game".into(),
            ..Default::default()
        };
        let (itx, irx) = channel();
        let (otx, orx) = channel();
        itx.send(PipelineEvent::StepStarted(0)).unwrap();
        itx.send(PipelineEvent::StepLog(0, "compiling".into(), false))
            .unwrap();
        itx.send(PipelineEvent::StepFinished(0, StepStatus::Failed))
            .unwrap();
        itx.send(PipelineEvent::Finished(PipelineOutcome::Failed))
            .unwrap();
        drop(itx);
        let mut saved = None;
        let ctx = egui::Context::default();
        record(new_run(&project, &pipeline), irx, otx, &ctx, &mut |run| {
            saved = Some(run.clone())
        });
        let run = saved.expect("run saved");
        assert_eq!(run.pipeline_name, "Nightly");
        assert_eq!(run.outcome, RunOutcome::Failed);
        assert_eq!(run.steps[0].outcome, StepOutcome::Failed);
        assert_eq!(run.steps[0].log, vec!["compiling".to_string()]);
        assert_eq!(run.steps[1].outcome, StepOutcome::NotRun);
        let forwarded: Vec<PipelineEvent> = orx.try_iter().collect();
        assert_eq!(forwarded.len(), 4);
        assert!(matches!(
            forwarded.last(),
            Some(PipelineEvent::Finished(PipelineOutcome::Failed))
        ));
    }

    #[test]
    fn failure_only_steps_run_after_a_failure_and_others_stop() {
        let mut normal = step("upload", true);
        let mut on_fail = step("alert", true);
        on_fail.run_if = RunIf::PreviousFailed;
        assert!(wanted(&normal, false));
        assert!(!wanted(&normal, true));
        assert!(!wanted(&on_fail, false));
        assert!(wanted(&on_fail, true));
        normal.enabled = false;
        assert!(!wanted(&normal, false));
        let mut changed = step("upload", true);
        changed.run_if = RunIf::FilesChanged;
        assert!(wanted(&changed, false));
        assert!(!wanted(&changed, true));
    }

    #[test]
    fn planned_skips_disabled_and_respects_start() {
        let steps = vec![
            step("a", true),
            step("b", false),
            step("c", true),
            step("d", true),
        ];
        assert_eq!(planned_indices(&steps, 0), vec![0, 2, 3]);
        assert_eq!(planned_indices(&steps, 2), vec![2, 3]);
        assert_eq!(planned_indices(&steps, 4), Vec::<usize>::new());
    }
}
