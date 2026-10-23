use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use eframe::egui;

use super::exec::{base_dir, emit, parse_env, split_args};
use super::{Control, PipelineEvent, StepStatus};
use crate::model::{PipelineStep, Project};
use crate::steamcmd::{self, BuildEvent, BuildResult};
use crate::vdf::{self, BuildOptions};

pub(super) fn run_command(
    project: &Project,
    i: usize,
    step: &PipelineStep,
    control: &Control,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    if step.exe.trim().is_empty() {
        emit(tx, ctx, i, "No executable set for this step.", true);
        return StepStatus::Failed;
    }
    run_process(
        step.exe.trim(),
        &split_args(&step.args),
        &base_dir(project, step),
        step,
        i,
        control,
        ctx,
        tx,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_process(
    exe: &str,
    args: &[String],
    dir: &Path,
    step: &PipelineStep,
    i: usize,
    control: &Control,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    cmd.current_dir(dir);
    for (k, v) in parse_env(&step.env) {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            emit(tx, ctx, i, format!("Could not launch {exe}: {e}"), true);
            return StepStatus::Failed;
        }
    };
    if let Ok(mut pid) = control.child_pid.lock() {
        *pid = Some(child.id());
    }
    let (ltx, lrx) = channel::<(bool, String)>();
    let readers = [
        child
            .stdout
            .take()
            .map(|r| spawn_lines(r, ltx.clone(), false)),
        child
            .stderr
            .take()
            .map(|r| spawn_lines(r, ltx.clone(), true)),
    ];
    drop(ltx);

    let start = Instant::now();
    let timeout = (step.timeout_secs > 0).then(|| Duration::from_secs(step.timeout_secs as u64));
    let status = loop {
        while let Ok((err, line)) = lrx.try_recv() {
            emit(tx, ctx, i, line, err);
        }
        if control.cancelled.load(Ordering::SeqCst) {
            let _ = child.kill();
            break StepStatus::Failed;
        }
        if let Some(t) = timeout
            && start.elapsed() > t
        {
            emit(tx, ctx, i, "Step timed out.", true);
            let _ = child.kill();
            break StepStatus::Failed;
        }
        match child.try_wait() {
            Ok(Some(exit)) => {
                while let Ok((err, line)) = lrx.try_recv() {
                    emit(tx, ctx, i, line, err);
                }
                break if exit.success() {
                    StepStatus::Success
                } else {
                    emit(tx, ctx, i, format!("Exited with {exit}."), true);
                    StepStatus::Failed
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(60)),
            Err(_) => break StepStatus::Failed,
        }
    };
    for r in readers.into_iter().flatten() {
        let _ = r.join();
    }
    if let Ok(mut pid) = control.child_pid.lock() {
        *pid = None;
    }
    status
}

fn spawn_lines<R: Read + Send + 'static>(
    reader: R,
    tx: Sender<(bool, String)>,
    err: bool,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let buf = BufReader::new(reader);
        for line in buf.lines().map_while(Result::ok) {
            if tx.send((err, line)).is_err() {
                break;
            }
        }
    })
}

pub(super) fn run_clean(
    project: &Project,
    i: usize,
    step: &PipelineStep,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    let base = base_dir(project, step);
    let mut ok = true;
    for target in step
        .targets
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
    {
        if target.contains(['*', '?', '[']) {
            ok &= clean_glob(&base, target, i, ctx, tx);
        } else {
            let path = base.join(target);
            match remove_path(&path) {
                Ok(true) => emit(tx, ctx, i, format!("Removed {}", path.display()), false),
                Ok(false) => emit(tx, ctx, i, format!("Not found: {}", path.display()), false),
                Err(e) => {
                    emit(
                        tx,
                        ctx,
                        i,
                        format!("Could not remove {}: {e}", path.display()),
                        true,
                    );
                    ok = false;
                }
            }
        }
    }
    if ok {
        StepStatus::Success
    } else {
        StepStatus::Failed
    }
}

fn remove_path(path: &Path) -> std::io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(true)
}

fn clean_glob(
    base: &Path,
    pattern: &str,
    i: usize,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> bool {
    let Ok(glob) = globset::Glob::new(pattern) else {
        emit(tx, ctx, i, format!("Invalid pattern: {pattern}"), true);
        return false;
    };
    let matcher = glob.compile_matcher();
    let Ok(entries) = std::fs::read_dir(base) else {
        return true;
    };
    let mut ok = true;
    for entry in entries.flatten() {
        if matcher.is_match(entry.file_name().to_string_lossy().as_ref()) {
            match remove_path(&entry.path()) {
                Ok(_) => emit(
                    tx,
                    ctx,
                    i,
                    format!("Removed {}", entry.path().display()),
                    false,
                ),
                Err(e) => {
                    emit(tx, ctx, i, format!("Could not remove: {e}"), true);
                    ok = false;
                }
            }
        }
    }
    ok
}

pub(super) fn run_upload(
    project: &Project,
    i: usize,
    control: &Control,
    ctx: &egui::Context,
    tx: &Sender<PipelineEvent>,
) -> StepStatus {
    let work = match crate::config::build_dir() {
        Ok(w) => w,
        Err(e) => {
            emit(
                tx,
                ctx,
                i,
                format!("Could not prepare build dir: {e}"),
                true,
            );
            return StepStatus::Failed;
        }
    };
    let paths = match vdf::write_vdfs(project, &BuildOptions::default(), &work) {
        Ok(p) => p,
        Err(e) => {
            emit(tx, ctx, i, format!("Could not write VDFs: {e}"), true);
            return StepStatus::Failed;
        }
    };
    let Some(app_build) = paths
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("appbuild.vdf"))
        .cloned()
    else {
        emit(tx, ctx, i, "No appbuild.vdf generated.", true);
        return StepStatus::Failed;
    };
    let (btx, brx) = channel();
    let options = steamcmd::RunOptions {
        log_path: Some(app_build.with_file_name("pipeline-build.log")),
        app_build_vdf: app_build,
        interactive: false,
    };
    let handle = match steamcmd::start_build(project, &options, ctx, btx) {
        Ok(h) => h,
        Err(e) => {
            emit(tx, ctx, i, format!("Could not start steamcmd: {e}"), true);
            return StepStatus::Failed;
        }
    };
    loop {
        if control.cancelled.load(Ordering::SeqCst) {
            steamcmd::cancel(&handle);
            return StepStatus::Failed;
        }
        match brx.recv_timeout(Duration::from_millis(120)) {
            Ok(BuildEvent::Log(line)) => {
                let err = line.to_ascii_lowercase().contains("error") || line.contains("FAILED");
                emit(tx, ctx, i, line, err);
            }
            Ok(BuildEvent::Prompt(kind)) => {
                let what = match kind {
                    steamcmd::PromptKind::Password => "a password",
                    steamcmd::PromptKind::SteamGuard => "a Steam Guard code",
                };
                emit(
                    tx,
                    ctx,
                    i,
                    format!(
                        "SteamCMD is asking for {what}. Authorize this account on the Build page first; background builds cannot answer prompts."
                    ),
                    true,
                );
                steamcmd::cancel(&handle);
                return StepStatus::Failed;
            }
            Ok(BuildEvent::Finished(res)) => {
                return match res {
                    BuildResult::Success { .. } => {
                        match super::changes::record_baseline(project) {
                            Ok(()) => emit(
                                tx,
                                ctx,
                                i,
                                "Saved a file snapshot for change detection.",
                                false,
                            ),
                            Err(e) => emit(
                                tx,
                                ctx,
                                i,
                                format!("Could not save a file snapshot: {e}"),
                                true,
                            ),
                        }
                        StepStatus::Success
                    }
                    _ => StepStatus::Failed,
                };
            }
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return StepStatus::Failed,
        }
    }
}
