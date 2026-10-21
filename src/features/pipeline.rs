#![allow(dead_code)]

mod actions;
mod exec;

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};

use eframe::egui;

use crate::model::{PipelineStep, Project};

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
    start_index: usize,
    ctx: &egui::Context,
    tx: Sender<PipelineEvent>,
) -> PipelineHandle {
    let control = Arc::new(Control::default());
    let project = project.clone();
    let ctx = ctx.clone();
    let worker = control.clone();
    std::thread::spawn(move || {
        let outcome = run_steps(&project, start_index, &worker, &ctx, &tx);
        let _ = tx.send(PipelineEvent::Finished(outcome));
        ctx.request_repaint();
    });
    PipelineHandle { control }
}

fn run_steps(
    project: &Project,
    start_index: usize,
    control: &Control,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> PipelineOutcome {
    for (i, step) in project.pipeline.iter().enumerate() {
        if i < start_index {
            continue;
        }
        if control.cancelled.load(Ordering::SeqCst) {
            return PipelineOutcome::Cancelled;
        }
        if !step.enabled {
            let _ = tx.send(PipelineEvent::StepFinished(i, StepStatus::Skipped));
            continue;
        }
        let _ = tx.send(PipelineEvent::StepStarted(i));
        ctx.request_repaint();
        let status = exec::run_step(project, i, step, control, ctx, tx);
        let _ = tx.send(PipelineEvent::StepFinished(i, status));
        ctx.request_repaint();
        if control.cancelled.load(Ordering::SeqCst) {
            return PipelineOutcome::Cancelled;
        }
        if status == StepStatus::Failed && !step.continue_on_failure {
            return PipelineOutcome::Failed;
        }
    }
    if control.cancelled.load(Ordering::SeqCst) {
        PipelineOutcome::Cancelled
    } else {
        PipelineOutcome::Completed
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
