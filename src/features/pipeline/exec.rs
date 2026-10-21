use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;

use eframe::egui;

use super::actions;
use super::{Control, PipelineEvent, StepStatus};
use crate::model::{PipelineStep, Project, StepKind};

pub(super) fn emit(
    tx: &Sender<PipelineEvent>,
    ctx: &egui::Context,
    i: usize,
    line: impl Into<String>,
    err: bool,
) {
    let _ = tx.send(PipelineEvent::StepLog(i, line.into(), err));
    ctx.request_repaint();
}

pub(super) fn run_step(
    project: &Project,
    i: usize,
    step: &PipelineStep,
    control: &Control,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    match step.kind {
        StepKind::RunCommand => actions::run_command(project, i, step, control, ctx, tx),
        StepKind::Clean => actions::run_clean(project, i, step, ctx, tx),
        StepKind::Upload => actions::run_upload(project, i, control, ctx, tx),
        StepKind::Notify => {
            emit(tx, ctx, i, format!("Notify: {}", step.message), false);
            let _ = tx.send(PipelineEvent::Notify(step.message.clone()));
            emit(
                tx,
                ctx,
                i,
                "Discord notifications are not configured yet (Q12).",
                false,
            );
            StepStatus::Success
        }
        StepKind::Confirm => {
            if let Ok(mut flag) = control.confirm_flag.lock() {
                *flag = false;
            }
            let _ = tx.send(PipelineEvent::AwaitConfirm(i, step.message.clone()));
            ctx.request_repaint();
            wait_confirm(control);
            if control.cancelled.load(Ordering::SeqCst) {
                StepStatus::Failed
            } else {
                emit(tx, ctx, i, "Confirmed.", false);
                StepStatus::Success
            }
        }
        StepKind::Backup => run_backup(project, i, ctx, tx),
    }
}

fn run_backup(
    project: &Project,
    i: usize,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    let settings = crate::config::load_settings().unwrap_or_default();
    if settings.backup_dir.trim().is_empty() {
        emit(
            tx,
            ctx,
            i,
            "No backup folder is set in Settings; skipping backup.",
            true,
        );
        return StepStatus::Skipped;
    }
    emit(tx, ctx, i, "Creating backup...", false);
    let root = PathBuf::from(&settings.backup_dir);
    let keep = settings.backup_keep.max(1);
    match crate::features::backup::create_backup(
        project,
        &root,
        project.build_counter,
        keep,
        settings.backup_auto_prune,
        None,
    ) {
        Ok(info) => {
            emit(
                tx,
                ctx,
                i,
                format!(
                    "Backed up to {} ({})",
                    info.path.display(),
                    crate::validate::human_size(info.size)
                ),
                false,
            );
            StepStatus::Success
        }
        Err(e) => {
            emit(tx, ctx, i, format!("Backup failed: {e}"), true);
            StepStatus::Failed
        }
    }
}

fn wait_confirm(control: &Control) {
    let mut flag = control
        .confirm_flag
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    while !*flag {
        flag = control
            .confirm_cv
            .wait(flag)
            .unwrap_or_else(|e| e.into_inner());
    }
}

pub(super) fn base_dir(project: &Project, step: &PipelineStep) -> PathBuf {
    let wd = step.working_dir.trim();
    if wd.is_empty() || wd == "." {
        project.content_root.clone()
    } else {
        let p = PathBuf::from(wd);
        if p.is_absolute() {
            p
        } else {
            project.content_root.join(p)
        }
    }
}

pub(super) fn split_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in input.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub(super) fn parse_env(input: &str) -> Vec<(String, String)> {
    input
        .lines()
        .filter_map(|line| {
            line.trim()
                .split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_args_handles_quotes() {
        assert_eq!(
            split_args("--path . --export \"Windows Desktop\" out.exe"),
            vec!["--path", ".", "--export", "Windows Desktop", "out.exe"]
        );
    }

    #[test]
    fn parse_env_pairs() {
        assert_eq!(
            parse_env("FOO=bar\n BAZ = qux \n\nBAD"),
            vec![
                ("FOO".to_string(), "bar".to_string()),
                ("BAZ".to_string(), "qux".to_string())
            ]
        );
    }
}
