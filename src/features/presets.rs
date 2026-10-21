#![allow(dead_code)]

use std::path::Path;

use crate::model::{PipelineStep, StepKind};

pub const TEMPLATE_NOTE: &str =
    "These are starting templates to review; the exact flags vary per project and engine setup.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Godot,
    Unity,
    Unreal,
}

impl Engine {
    pub fn label(self) -> &'static str {
        match self {
            Engine::Godot => "Godot",
            Engine::Unity => "Unity",
            Engine::Unreal => "Unreal",
        }
    }
}

pub fn detect(content_root: &Path) -> Option<Engine> {
    if content_root.as_os_str().is_empty() {
        return None;
    }
    if content_root.join("project.godot").is_file() {
        return Some(Engine::Godot);
    }
    if content_root
        .join("ProjectSettings")
        .join("ProjectVersion.txt")
        .is_file()
    {
        return Some(Engine::Unity);
    }
    if let Ok(entries) = std::fs::read_dir(content_root) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("uproject") {
                return Some(Engine::Unreal);
            }
        }
    }
    None
}

fn new_step(name: &str, kind: StepKind) -> PipelineStep {
    PipelineStep {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        kind,
        enabled: true,
        ..Default::default()
    }
}

fn command(name: &str, exe: &str, args: &str) -> PipelineStep {
    let mut step = new_step(name, StepKind::RunCommand);
    step.exe = exe.to_string();
    step.args = args.to_string();
    step.working_dir = ".".to_string();
    step
}

pub fn upload_step() -> PipelineStep {
    new_step("Upload to Steam", StepKind::Upload)
}

pub fn preset_steps(engine: Engine) -> Vec<PipelineStep> {
    let build = match engine {
        Engine::Godot => command(
            "Export (Godot)",
            "godot",
            "--headless --path . --export-release \"Windows Desktop\" build/game.exe",
        ),
        Engine::Unity => command(
            "Build (Unity)",
            "Unity.exe",
            "-batchmode -quit -projectPath . -buildTarget Win64 -executeMethod BuildScript.PerformBuild -logFile -",
        ),
        Engine::Unreal => command(
            "BuildCookRun (Unreal)",
            "RunUAT.bat",
            "BuildCookRun -project=\"MyGame.uproject\" -noP4 -platform=Win64 -clientconfig=Shipping -cook -build -stage -pak -archive -archivedirectory=build",
        ),
    };
    vec![build, upload_step()]
}
