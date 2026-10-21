use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

use eframe::egui;

use crate::features::manifest;
use crate::features::pipeline::{self, PipelineEvent, PipelineHandle, PipelineOutcome, StepStatus};
use crate::model::{BuildStatus, Project};
use crate::steamcmd::{self, BuildEvent, BuildResult, Stage};
use crate::validate::{human_size, validate_project};
use crate::vdf::{self, BuildOptions};

pub const SUCCESS: i32 = 0;
pub const FAILURE: i32 = 1;
pub const VALIDATION: i32 = 2;
pub const LOGIN: i32 = 3;
pub const CANCELLED: i32 = 130;

fn stage_label(stage: Stage) -> &'static str {
    match stage {
        Stage::Connecting => "Connecting",
        Stage::LoggingIn => "Logging in",
        Stage::Scanning => "Scanning",
        Stage::Uploading => "Uploading",
        Stage::Committing => "Committing",
        Stage::Finished => "Finished",
    }
}

fn step_label(status: StepStatus) -> &'static str {
    match status {
        StepStatus::Success => "ok",
        StepStatus::Failed => "failed",
        StepStatus::Skipped => "skipped",
    }
}

fn exit_for(result: &BuildResult) -> i32 {
    match result {
        BuildResult::Success { .. } => SUCCESS,
        BuildResult::Failure { .. } => FAILURE,
        BuildResult::Cancelled => CANCELLED,
    }
}

fn drive(rx: Receiver<BuildEvent>, mut cancel: impl FnMut()) -> i32 {
    while let Ok(event) = rx.recv() {
        match event {
            BuildEvent::Log(line) => println!("{line}"),
            BuildEvent::Stage(stage) => println!("» {}", stage_label(stage)),
            BuildEvent::Progress(_) => {}
            BuildEvent::Prompt(_) => {
                eprintln!("Steam login or Steam Guard is required; not available in CLI mode.");
                cancel();
                return LOGIN;
            }
            BuildEvent::Finished(result) => return exit_for(&result),
        }
    }
    FAILURE
}

fn drive_pipeline(rx: Receiver<PipelineEvent>, handle: &PipelineHandle) -> i32 {
    while let Ok(event) = rx.recv() {
        match event {
            PipelineEvent::StepStarted(i) => println!("» step {}", i + 1),
            PipelineEvent::StepLog(_, line, _) => println!("{line}"),
            PipelineEvent::StepFinished(i, status) => {
                println!("  step {} {}", i + 1, step_label(status))
            }
            PipelineEvent::AwaitConfirm(_, message) => {
                eprintln!(
                    "Confirmation step '{message}' needs interaction; not available in CLI mode."
                );
                handle.cancel();
            }
            PipelineEvent::Notify(line) => println!("{line}"),
            PipelineEvent::Finished(outcome) => {
                return match outcome {
                    PipelineOutcome::Completed => SUCCESS,
                    PipelineOutcome::Failed => FAILURE,
                    PipelineOutcome::Cancelled => CANCELLED,
                };
            }
        }
    }
    FAILURE
}

fn last_manifest_path(project: &Project, branch: &str) -> Option<PathBuf> {
    let conn = crate::history::connect().ok()?;
    let records = crate::history::all(&conn).ok()?;
    records
        .into_iter()
        .find(|r| {
            r.project_id == project.id
                && r.branch == branch
                && r.status == BuildStatus::Success
                && r.manifest_path.is_some()
        })
        .and_then(|r| r.manifest_path)
}

fn print_diff(project: &Project, branch: &str) {
    let previous = last_manifest_path(project, branch);
    let current = manifest::build_manifest(project, None);
    let prev = previous.and_then(|p| manifest::load_manifest(&p).ok());
    let summary = manifest::diff(&current, prev.as_ref());
    println!(
        "Changes: +{} ~{} -{} ={}  (~{} to upload, estimate)",
        summary.added,
        summary.modified,
        summary.removed,
        summary.unchanged,
        human_size(summary.estimated_upload()),
    );
}

pub fn execute_build(
    project: &Project,
    branch: &str,
    desc: &str,
    preview: bool,
    no_diff: bool,
) -> i32 {
    let errors = validate_project(project);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("validation: {} — {}", e.field, e.message);
        }
        return VALIDATION;
    }

    let shown_branch = if branch.is_empty() { "default" } else { branch };
    println!("Building {} on branch '{}'", project.name, shown_branch);
    if !desc.trim().is_empty() {
        println!("Description: {desc}");
    }
    if !no_diff {
        print_diff(project, branch);
    }

    let ctx = egui::Context::default();
    if !project.pipeline.is_empty() {
        let (tx, rx) = channel();
        let handle = pipeline::start(project, 0, &ctx, tx);
        return drive_pipeline(rx, &handle);
    }

    let work = match crate::config::build_dir() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("could not prepare build directory: {e}");
            return FAILURE;
        }
    };
    let options = BuildOptions {
        preview,
        local: None,
    };
    let paths = match vdf::write_vdfs(project, &options, &work) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("could not write VDFs: {e}");
            return FAILURE;
        }
    };
    let Some(app_build) = paths
        .into_iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()) == Some("appbuild.vdf"))
    else {
        eprintln!("no appbuild.vdf was generated");
        return FAILURE;
    };

    let (tx, rx) = channel();
    let opts = steamcmd::RunOptions {
        app_build_vdf: app_build,
        log_path: None,
        interactive: false,
    };
    let handle = match steamcmd::start_build(project, &opts, &ctx, tx) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("could not start steamcmd: {e}");
            return FAILURE;
        }
    };
    drive(rx, || steamcmd::cancel(&handle))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_bin() -> PathBuf {
        if let Some(p) = option_env!("CARGO_BIN_EXE_fake_steamcmd") {
            return PathBuf::from(p);
        }
        let mut exe = std::env::current_exe().expect("current exe");
        exe.pop();
        if exe.file_name().is_some_and(|n| n == "deps") {
            exe.pop();
        }
        exe.push(format!("fake_steamcmd{}", std::env::consts::EXE_SUFFIX));
        exe
    }

    fn valid_project(steamcmd: PathBuf) -> Project {
        Project {
            name: "Game".into(),
            app_id: "480".into(),
            content_root: std::env::temp_dir(),
            build_output_dir: std::env::temp_dir(),
            steamcmd_path: steamcmd,
            steam_username: "tester".into(),
            ..Default::default()
        }
    }

    #[test]
    fn drive_maps_outcomes_and_prompt() {
        for (event, code) in [
            (
                BuildEvent::Finished(BuildResult::Success {
                    app_id: "480".into(),
                }),
                SUCCESS,
            ),
            (
                BuildEvent::Finished(BuildResult::Failure {
                    message: "x".into(),
                }),
                FAILURE,
            ),
            (BuildEvent::Finished(BuildResult::Cancelled), CANCELLED),
        ] {
            let (tx, rx) = channel();
            tx.send(event).unwrap();
            drop(tx);
            assert_eq!(drive(rx, || {}), code);
        }
        let (tx, rx) = channel();
        tx.send(BuildEvent::Prompt(steamcmd::PromptKind::SteamGuard))
            .unwrap();
        drop(tx);
        let mut cancelled = false;
        assert_eq!(drive(rx, || cancelled = true), LOGIN);
        assert!(cancelled);
    }

    #[test]
    fn execute_build_against_fake_succeeds() {
        let project = valid_project(fake_bin());
        assert_eq!(
            execute_build(&project, "", "from test", false, true),
            SUCCESS
        );
    }

    #[test]
    fn execute_build_reports_validation_errors() {
        let project = Project {
            steamcmd_path: fake_bin(),
            ..Default::default()
        };
        assert_eq!(execute_build(&project, "", "", false, true), VALIDATION);
    }

    #[test]
    fn execute_build_fails_when_steamcmd_missing() {
        let project = valid_project(PathBuf::from("/nonexistent/steamcmd-xyz"));
        assert_eq!(execute_build(&project, "", "", false, true), FAILURE);
    }
}
