#![allow(dead_code)]

use std::path::{Path, PathBuf};

use crate::model::{PipelineStep, Project, StepKind};

pub const TEMPLATE_NOTE: &str =
    "These are starting templates to review; the exact flags vary per project and engine setup.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Godot,
    Unity,
    Unreal,
}

impl Engine {
    pub const ALL: [Engine; 3] = [Engine::Unity, Engine::Unreal, Engine::Godot];

    pub fn label(self) -> &'static str {
        match self {
            Engine::Godot => "Godot",
            Engine::Unity => "Unity",
            Engine::Unreal => "Unreal",
        }
    }

    pub fn from_label(label: &str) -> Option<Engine> {
        Engine::ALL
            .into_iter()
            .find(|e| e.label().eq_ignore_ascii_case(label.trim()))
    }

    pub fn exe_hint(self) -> &'static str {
        match self {
            Engine::Unity => r"C:\Program Files\Unity\Hub\Editor\<version>\Editor\Unity.exe",
            Engine::Unreal => {
                r"C:\Program Files\Epic Games\UE_5.4\Engine\Build\BatchFiles\RunUAT.bat"
            }
            Engine::Godot => "godot (on PATH) or the full path to the Godot executable",
        }
    }
}

pub const PLATFORMS: [&str; 3] = ["Windows", "Linux", "macOS"];

pub struct GameBuild {
    pub exe: String,
    pub args: Vec<String>,
    pub dir: PathBuf,
}

impl GameBuild {
    pub fn display(&self) -> String {
        std::iter::once(&self.exe)
            .chain(self.args.iter())
            .map(|a| {
                if a.contains(' ') {
                    format!("\"{a}\"")
                } else {
                    a.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn step_engine(project: &Project, step: &PipelineStep) -> Option<Engine> {
    Engine::from_label(&step.engine).or_else(|| detect(&project_dir(project, step)))
}

fn project_dir(project: &Project, step: &PipelineStep) -> PathBuf {
    let root = crate::features::maintenance::resolve(project).content_root;
    let wd = step.working_dir.trim();
    if wd.is_empty() || wd == "." {
        root
    } else if Path::new(wd).is_absolute() {
        PathBuf::from(wd)
    } else {
        root.join(wd)
    }
}

fn output_dir(project: &Project, step: &PipelineStep) -> Option<PathBuf> {
    let out = step.output.trim();
    if !out.is_empty() {
        return Some(PathBuf::from(out));
    }
    project
        .depots
        .first()
        .map(|d| d.local_path.clone())
        .filter(|p| !p.as_os_str().is_empty())
}

fn game_name(project: &Project) -> String {
    let name: String = project
        .name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if name.is_empty() {
        "Game".to_string()
    } else {
        name
    }
}

fn platform_of(step: &PipelineStep) -> &'static str {
    PLATFORMS
        .into_iter()
        .find(|p| p.eq_ignore_ascii_case(step.platform.trim()))
        .unwrap_or("Windows")
}

pub fn default_export_preset(platform: &str) -> &'static str {
    match platform {
        "Linux" => "Linux",
        "macOS" => "macOS",
        _ => "Windows Desktop",
    }
}

pub fn build_command(project: &Project, step: &PipelineStep) -> Result<GameBuild, String> {
    let dir = project_dir(project, step);
    let engine = step_engine(project, step).ok_or_else(|| {
        format!(
            "Could not detect a Unity, Unreal or Godot project in {}. Pick the engine or set the project folder.",
            dir.display()
        )
    })?;
    let out = output_dir(project, step)
        .ok_or("Set an output folder, or add a depot so the build can go into its folder.")?;
    let platform = platform_of(step);
    let name = game_name(project);
    let exe = step.exe.trim().to_string();
    let path = |p: PathBuf| p.to_string_lossy().to_string();
    let mut args: Vec<String> = match engine {
        Engine::Unity => {
            if exe.is_empty() {
                return Err("Set the path to the Unity editor (Unity.exe).".into());
            }
            let (target, flag, file) = match platform {
                "Linux" => ("Linux64", "-buildLinux64Player", format!("{name}.x86_64")),
                "macOS" => (
                    "OSXUniversal",
                    "-buildOSXUniversalPlayer",
                    format!("{name}.app"),
                ),
                _ => ("Win64", "-buildWindows64Player", format!("{name}.exe")),
            };
            vec![
                "-batchmode".into(),
                "-nographics".into(),
                "-quit".into(),
                "-projectPath".into(),
                path(dir.clone()),
                "-buildTarget".into(),
                target.into(),
                flag.into(),
                path(out.join(file)),
                "-logFile".into(),
                "-".into(),
            ]
        }
        Engine::Unreal => {
            if exe.is_empty() {
                return Err("Set the path to RunUAT.bat in your Unreal Engine install.".into());
            }
            let uproject = std::fs::read_dir(&dir)
                .ok()
                .and_then(|entries| {
                    entries
                        .flatten()
                        .map(|e| e.path())
                        .find(|p| p.extension().and_then(|x| x.to_str()) == Some("uproject"))
                })
                .ok_or_else(|| format!("No .uproject file found in {}", dir.display()))?;
            let target = match platform {
                "Linux" => "Linux",
                "macOS" => "Mac",
                _ => "Win64",
            };
            vec![
                "BuildCookRun".into(),
                format!("-project={}", path(uproject)),
                "-noP4".into(),
                format!("-platform={target}"),
                "-clientconfig=Shipping".into(),
                "-build".into(),
                "-cook".into(),
                "-stage".into(),
                "-pak".into(),
                "-archive".into(),
                format!("-archivedirectory={}", path(out)),
                "-utf8output".into(),
            ]
        }
        Engine::Godot => {
            let preset = if step.export_preset.trim().is_empty() {
                default_export_preset(platform).to_string()
            } else {
                step.export_preset.trim().to_string()
            };
            let file = match platform {
                "Linux" => format!("{name}.x86_64"),
                "macOS" => format!("{name}.zip"),
                _ => format!("{name}.exe"),
            };
            vec![
                "--headless".into(),
                "--path".into(),
                path(dir.clone()),
                "--export-release".into(),
                preset,
                path(out.join(file)),
            ]
        }
    };
    args.extend(crate::features::pipeline::split_args(&step.args));
    let exe = if exe.is_empty() {
        "godot".to_string()
    } else {
        exe
    };
    Ok(GameBuild { exe, args, dir })
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

pub fn build_step(engine: Engine) -> PipelineStep {
    let mut step = new_step(&format!("Build ({})", engine.label()), StepKind::BuildGame);
    step.engine = engine.label().to_string();
    step.platform = "Windows".to_string();
    step.working_dir = ".".to_string();
    if engine == Engine::Godot {
        step.exe = "godot".to_string();
    }
    step
}

pub fn preset_steps(engine: Engine) -> Vec<PipelineStep> {
    vec![build_step(engine), upload_step()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Depot;

    fn project(root: &Path) -> Project {
        Project {
            name: "My Game".into(),
            content_root: root.to_path_buf(),
            depots: vec![Depot {
                local_path: root.join("depot"),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn unity_windows_build_targets_the_first_depot_folder() {
        let root = std::env::temp_dir().join("onyx_unity_cmd");
        let mut step = build_step(Engine::Unity);
        step.exe = "Unity.exe".into();
        let cmd = build_command(&project(&root), &step).expect("command");
        assert_eq!(cmd.exe, "Unity.exe");
        let exe_path = root
            .join("depot")
            .join("MyGame.exe")
            .to_string_lossy()
            .to_string();
        let flag = cmd
            .args
            .iter()
            .position(|a| a == "-buildWindows64Player")
            .expect("flag");
        assert_eq!(cmd.args[flag + 1], exe_path);
        assert!(cmd.args.contains(&"-batchmode".to_string()));
    }

    #[test]
    fn godot_uses_the_platform_export_preset_and_extra_args() {
        let root = std::env::temp_dir().join("onyx_godot_cmd");
        let mut step = build_step(Engine::Godot);
        step.platform = "Linux".into();
        step.args = "--verbose".into();
        let cmd = build_command(&project(&root), &step).expect("command");
        assert_eq!(cmd.exe, "godot");
        assert!(cmd.args.contains(&"Linux".to_string()));
        assert!(cmd.args.last().is_some_and(|a| a == "--verbose"));
        assert!(cmd.args.iter().any(|a| a.ends_with("MyGame.x86_64")));
    }

    #[test]
    fn unreal_finds_the_uproject_and_requires_runuat() {
        let root = std::env::temp_dir().join(format!("onyx_unreal_cmd_{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("dir");
        std::fs::write(root.join("Shooter.uproject"), "{}").expect("uproject");
        let mut step = build_step(Engine::Unreal);
        assert!(build_command(&project(&root), &step).is_err());
        step.exe = "RunUAT.bat".into();
        let cmd = build_command(&project(&root), &step).expect("command");
        assert!(cmd.args.iter().any(|a| a.ends_with("Shooter.uproject")));
        assert!(cmd.args.contains(&"-platform=Win64".to_string()));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_output_and_depots_is_an_error() {
        let mut p = project(&std::env::temp_dir());
        p.depots.clear();
        let mut step = build_step(Engine::Godot);
        step.engine = "Godot".into();
        assert!(build_command(&p, &step).is_err());
    }
}
