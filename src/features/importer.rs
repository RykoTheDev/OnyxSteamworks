use std::path::{Path, PathBuf};

use crate::model::{Depot, Project};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Godot,
    Unity,
    Unreal,
    Unknown,
}

impl Engine {
    pub fn label(self) -> &'static str {
        match self {
            Engine::Godot => "Godot",
            Engine::Unity => "Unity",
            Engine::Unreal => "Unreal",
            Engine::Unknown => "Unknown engine",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Mac,
    Linux,
}

impl Os {
    pub fn label(self) -> &'static str {
        match self {
            Os::Windows => "Windows",
            Os::Mac => "macOS",
            Os::Linux => "Linux",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DetectedDepot {
    pub name: String,
    pub os: Option<Os>,
    pub local_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Detected {
    pub name: String,
    pub engine: Engine,
    pub content_root: PathBuf,
    pub depots: Vec<DetectedDepot>,
    pub executables: Vec<String>,
}

pub fn classify_os(name: &str) -> Option<Os> {
    let n = name.to_ascii_lowercase();
    if n.contains("win") {
        Some(Os::Windows)
    } else if n.contains("mac") || n.contains("osx") || n.contains("darwin") {
        Some(Os::Mac)
    } else if n.contains("linux") {
        Some(Os::Linux)
    } else {
        None
    }
}

fn is_executable(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".exe") || n.ends_with(".x86_64") || n.ends_with(".app") || n.ends_with(".sh")
}

fn engine_from_names(names: &[String]) -> Engine {
    if names.iter().any(|n| n == "project.godot") {
        Engine::Godot
    } else if names.iter().any(|n| n == "ProjectSettings") {
        Engine::Unity
    } else if names
        .iter()
        .any(|n| n.to_ascii_lowercase().ends_with(".uproject"))
    {
        Engine::Unreal
    } else {
        Engine::Unknown
    }
}

fn child_names(dir: &Path) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            out.push((name, is_dir));
        }
    }
    out
}

const OUTPUT_HINTS: [&str; 6] = ["build", "builds", "dist", "export", "exports", "output"];

pub fn detect(root: &Path) -> Detected {
    let top = child_names(root);
    let names: Vec<String> = top.iter().map(|(n, _)| n.clone()).collect();
    let engine = engine_from_names(&names);

    let mut content_root = root.to_path_buf();
    if let Some((dir, _)) = top
        .iter()
        .find(|(n, d)| *d && OUTPUT_HINTS.contains(&n.to_ascii_lowercase().as_str()))
    {
        content_root = root.join(dir);
    }

    let dirs: Vec<(String, Option<Os>)> = child_names(&content_root)
        .into_iter()
        .filter(|(_, d)| *d)
        .map(|(n, _)| {
            let os = classify_os(&n);
            (n, os)
        })
        .collect();

    let mut depots = Vec::new();
    let os_dirs: Vec<&(String, Option<Os>)> = dirs.iter().filter(|(_, os)| os.is_some()).collect();
    if os_dirs.is_empty() {
        depots.push(DetectedDepot {
            name: "content".to_string(),
            os: None,
            local_path: content_root.clone(),
        });
    } else {
        for (name, os) in &os_dirs {
            depots.push(DetectedDepot {
                name: name.clone(),
                os: *os,
                local_path: content_root.join(name),
            });
        }
    }

    let mut executables: Vec<String> = child_names(&content_root)
        .into_iter()
        .filter(|(n, d)| !*d && is_executable(n))
        .map(|(n, _)| n)
        .collect();
    executables.sort();
    executables.truncate(8);

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "New project".to_string());
    Detected {
        name,
        engine,
        content_root,
        depots,
        executables,
    }
}

pub fn to_project(d: &Detected) -> Project {
    let depots = d
        .depots
        .iter()
        .enumerate()
        .map(|(i, dd)| Depot {
            depot_id: format!("{}", 1001 + i),
            name: dd.name.clone(),
            local_path: dd.local_path.clone(),
            depot_path: ".".to_string(),
            recursive: true,
            exclusions: Vec::new(),
        })
        .collect();
    Project {
        id: uuid::Uuid::new_v4().to_string(),
        name: d.name.clone(),
        content_root: d.content_root.clone(),
        depots,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_os_matches_common_folder_names() {
        assert_eq!(classify_os("Windows"), Some(Os::Windows));
        assert_eq!(classify_os("win64"), Some(Os::Windows));
        assert_eq!(classify_os("macOS"), Some(Os::Mac));
        assert_eq!(classify_os("osx-universal"), Some(Os::Mac));
        assert_eq!(classify_os("LinuxServer"), Some(Os::Linux));
        assert_eq!(classify_os("assets"), None);
    }

    #[test]
    fn engine_detection_from_marker_files() {
        assert_eq!(
            engine_from_names(&["project.godot".into(), "scenes".into()]),
            Engine::Godot
        );
        assert_eq!(
            engine_from_names(&["ProjectSettings".into(), "Assets".into()]),
            Engine::Unity
        );
        assert_eq!(engine_from_names(&["Game.uproject".into()]), Engine::Unreal);
        assert_eq!(engine_from_names(&["readme.txt".into()]), Engine::Unknown);
    }

    #[test]
    fn detect_builds_os_depots_and_project() {
        let base = std::env::temp_dir().join(format!("onyx_imp_{}", uuid::Uuid::new_v4()));
        let build = base.join("build");
        std::fs::create_dir_all(build.join("windows")).unwrap();
        std::fs::create_dir_all(build.join("linux")).unwrap();
        std::fs::create_dir_all(base.join("src")).unwrap();
        std::fs::write(base.join("project.godot"), "[gd]").unwrap();

        let d = detect(&base);
        assert_eq!(d.engine, Engine::Godot);
        assert_eq!(d.content_root, build);
        assert_eq!(d.depots.len(), 2);
        let project = to_project(&d);
        assert_eq!(project.depots.len(), 2);
        assert_eq!(project.depots[0].depot_id, "1001");

        std::fs::remove_dir_all(&base).ok();
    }
}
