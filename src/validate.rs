#![allow(dead_code)]

use std::collections::HashSet;
use std::path::Path;

use walkdir::WalkDir;

use crate::model::Project;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field}: {message}")]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

fn is_numeric(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

pub fn validate_project(project: &Project) -> Vec<FieldError> {
    let mut errors = Vec::new();

    if project.name.trim().is_empty() {
        errors.push(FieldError::new("name", "Name is required"));
    }
    if !is_numeric(&project.app_id) {
        errors.push(FieldError::new("app_id", "App ID must be a number"));
    }
    if project.content_root.as_os_str().is_empty() {
        errors.push(FieldError::new("content_root", "Content root is required"));
    }
    if project.build_output_dir.as_os_str().is_empty() {
        errors.push(FieldError::new(
            "build_output_dir",
            "Build output is required",
        ));
    }
    if project.steamcmd_path.as_os_str().is_empty() {
        errors.push(FieldError::new(
            "steamcmd_path",
            "SteamCMD path is required",
        ));
    }

    let mut seen = HashSet::new();
    for (i, depot) in project.depots.iter().enumerate() {
        if !is_numeric(&depot.depot_id) {
            errors.push(FieldError::new(
                format!("depots[{i}].depot_id"),
                "Depot id must be a number",
            ));
        } else if !seen.insert(depot.depot_id.clone()) {
            errors.push(FieldError::new(
                format!("depots[{i}].depot_id"),
                format!("Duplicate depot id {}", depot.depot_id),
            ));
        }
    }

    errors
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fix {
    AddExclusion(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    pub fix: Option<Fix>,
}

impl Issue {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            fix: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            fix: None,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            message: message.into(),
            fix: None,
        }
    }

    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    pub fn is_blocking(&self) -> bool {
        self.severity == Severity::Error
    }
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn dir_totals(root: &Path, recursive: bool) -> (u64, u64) {
    let depth = if recursive { usize::MAX } else { 1 };
    let mut files = 0u64;
    let mut bytes = 0u64;
    for entry in WalkDir::new(root)
        .max_depth(depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            files += 1;
            bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    (files, bytes)
}

fn dir_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".steampipe_write_test");
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

pub fn preflight_checks(project: &Project) -> Vec<Issue> {
    let mut issues = Vec::new();

    if !project.steamcmd_path.is_file() {
        issues.push(Issue::error(
            "SteamCMD executable was not found at the configured path",
        ));
    }

    if project.content_root.as_os_str().is_empty() || !project.content_root.is_dir() {
        issues.push(Issue::error("Content root folder does not exist"));
    } else if dir_totals(&project.content_root, true).0 == 0 {
        issues.push(Issue::warning("Content root folder is empty"));
    }

    let mut seen = HashSet::new();
    let mut total = 0u64;
    for depot in &project.depots {
        let id = if depot.depot_id.is_empty() {
            "?".to_string()
        } else {
            depot.depot_id.clone()
        };
        if !is_numeric(&depot.depot_id) {
            issues.push(Issue::error(format!("Depot {id} id must be a number")));
        } else if !seen.insert(depot.depot_id.clone()) {
            issues.push(Issue::error(format!(
                "Depot {id} id is used by more than one depot"
            )));
        }
        if depot.local_path.as_os_str().is_empty() || !depot.local_path.is_dir() {
            issues.push(Issue::error(format!("Depot {id} folder does not exist")));
        } else {
            let (files, bytes) = dir_totals(&depot.local_path, depot.recursive);
            if files == 0 {
                issues.push(Issue::warning(format!("Depot {id} folder has no files")));
            }
            total += bytes;
        }
    }

    if project.build_output_dir.as_os_str().is_empty() || !dir_writable(&project.build_output_dir) {
        issues.push(Issue::error("Build output folder is not writable"));
    }

    if total > 0 {
        issues.push(Issue::info(format!(
            "Estimated upload size is {}",
            human_size(total)
        )));
    }

    let branch = project.set_live_branch.trim();
    if branch.is_empty() || branch.eq_ignore_ascii_case("default") {
        issues.push(Issue::warning(
            "The default branch cannot be set live through SteamCMD",
        ));
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Depot, Project};

    fn valid_project() -> Project {
        Project {
            name: "Game".to_string(),
            app_id: "480".to_string(),
            content_root: "C:/content".into(),
            build_output_dir: "C:/out".into(),
            steamcmd_path: "C:/steamcmd.exe".into(),
            ..Default::default()
        }
    }

    #[test]
    fn accepts_a_valid_project() {
        assert!(validate_project(&valid_project()).is_empty());
    }

    #[test]
    fn flags_bad_app_id_and_missing_paths() {
        let mut p = valid_project();
        p.app_id = "not-a-number".to_string();
        p.content_root = Default::default();
        let errors = validate_project(&p);
        assert!(errors.iter().any(|e| e.field == "app_id"));
        assert!(errors.iter().any(|e| e.field == "content_root"));
    }

    #[test]
    fn flags_non_numeric_and_duplicate_depot_ids() {
        let mut p = valid_project();
        p.depots = vec![
            Depot {
                depot_id: "481".into(),
                ..Default::default()
            },
            Depot {
                depot_id: "481".into(),
                ..Default::default()
            },
            Depot {
                depot_id: "abc".into(),
                ..Default::default()
            },
        ];
        let errors = validate_project(&p);
        assert!(errors.iter().any(|e| e.field == "depots[1].depot_id"));
        assert!(errors.iter().any(|e| e.field == "depots[2].depot_id"));
    }

    #[test]
    fn human_size_scales_units() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert!(human_size(3 * 1024 * 1024 * 1024).ends_with("GB"));
    }

    #[test]
    fn preflight_flags_missing_paths_as_errors() {
        let p = valid_project();
        let issues = preflight_checks(&p);
        assert!(
            issues
                .iter()
                .any(|i| i.is_blocking() && i.message.contains("SteamCMD"))
        );
        assert!(
            issues
                .iter()
                .any(|i| i.is_blocking() && i.message.contains("Content root"))
        );
    }

    #[test]
    fn preflight_warns_on_default_set_live_branch() {
        let dir = std::env::temp_dir();
        let mut p = valid_project();
        p.steamcmd_path = dir.join("nope.exe");
        p.set_live_branch = "default".into();
        let issues = preflight_checks(&p);
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Warning && i.message.contains("default branch"))
        );
    }
}
