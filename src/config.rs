//! On-disk storage for projects and settings: one JSON file per project plus a
//! settings file under the platform config directory. Writes are atomic and
//! loading is tolerant (unknown fields ignored, missing fields default).
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::model::Project;

/// Current on-disk schema version. Bump when the stored shape changes in a way
/// that needs migration; the tolerant loader handles additive changes already.
pub const SCHEMA_VERSION: u32 = 1;

/// Errors from the storage layer.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not determine the config directory")]
    NoConfigDir,
    #[error("project not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, ConfigError>;

/// Versioned envelope written to each project file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct ProjectFile {
    schema_version: u32,
    project: Project,
}

/// Persisted application settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub last_opened_project: Option<String>,
    pub remember_password: bool,
    pub skip_review: bool,
    pub backup_dir: String,
    pub backup_keep: u32,
    pub backup_auto_prune: bool,
    pub keep_verify_temp: bool,
    pub llm_enabled: bool,
    pub llm_base_url: String,
    pub llm_model: String,
    pub llm_api_key: String,
    pub advisor_large_mb: u32,
    pub analytics_growth_pct: u32,
    pub queue_max_retries: u32,
    pub accounts: Vec<String>,
    pub checklist_reset_on_live: bool,
    pub notify_toast: bool,
    pub notify_success: bool,
    pub notify_failure: bool,
    pub discord_enabled: bool,
    pub default_content: String,
    pub default_output: String,
    pub ui_scale: f32,
    pub reduce_motion: bool,
    pub log_retention_days: u32,
    pub use_fake_steamcmd: bool,
    pub steamcmd_override: String,
    pub wizard_done: bool,
}

/// Root config directory for the app, created if missing.
fn config_dir() -> Result<PathBuf> {
    let dirs =
        ProjectDirs::from("dev", "steampipe", "SteamPipe GUI").ok_or(ConfigError::NoConfigDir)?;
    let dir = dirs.config_dir().to_path_buf();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory holding one JSON file per project, created if missing.
fn projects_dir() -> Result<PathBuf> {
    let dir = config_dir()?.join("projects");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Root data directory for the app, created if missing.
pub fn data_dir() -> Result<PathBuf> {
    let dirs =
        ProjectDirs::from("dev", "steampipe", "SteamPipe GUI").ok_or(ConfigError::NoConfigDir)?;
    let dir = dirs.data_dir().to_path_buf();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory holding build logs, created if missing.
pub fn log_dir() -> Result<PathBuf> {
    let dirs =
        ProjectDirs::from("dev", "steampipe", "SteamPipe GUI").ok_or(ConfigError::NoConfigDir)?;
    let dir = dirs.data_dir().join("logs");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Directory holding generated VDFs for a build run, created if missing.
pub fn build_dir() -> Result<PathBuf> {
    let dirs =
        ProjectDirs::from("dev", "steampipe", "SteamPipe GUI").ok_or(ConfigError::NoConfigDir)?;
    let dir = dirs.data_dir().join("builds");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Write `bytes` to `path` atomically: a sibling temp file, then a rename.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn project_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

fn save_project_in(dir: &Path, project: &Project) -> Result<()> {
    let file = ProjectFile {
        schema_version: SCHEMA_VERSION,
        project: project.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&file)?;
    write_atomic(&project_path(dir, &project.id), &bytes)
}

fn load_project_in(dir: &Path, id: &str) -> Result<Project> {
    let path = project_path(dir, id);
    if !path.exists() {
        return Err(ConfigError::NotFound(id.to_string()));
    }
    let bytes = fs::read(&path)?;
    let file: ProjectFile = serde_json::from_slice(&bytes)?;
    Ok(file.project)
}

fn list_projects_in(dir: &Path) -> Result<Vec<Project>> {
    let mut projects = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match fs::read(&path)
            .map_err(ConfigError::from)
            .and_then(|b| serde_json::from_slice::<ProjectFile>(&b).map_err(ConfigError::from))
        {
            Ok(file) => projects.push(file.project),
            // A single corrupt file must not break the whole list.
            Err(e) => eprintln!("skipping unreadable project {}: {e}", path.display()),
        }
    }
    projects.sort_by_key(|p| p.name.to_lowercase());
    Ok(projects)
}

/// List every stored project, sorted by name.
pub fn list_projects() -> Result<Vec<Project>> {
    list_projects_in(&projects_dir()?)
}

/// Load a single project by id.
pub fn load_project(id: &str) -> Result<Project> {
    load_project_in(&projects_dir()?, id)
}

/// Save (create or overwrite) a project.
pub fn save_project(project: &Project) -> Result<()> {
    save_project_in(&projects_dir()?, project)
}

/// Delete a project by id. A missing file is treated as success.
pub fn delete_project(id: &str) -> Result<()> {
    let path = project_path(&projects_dir()?, id);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Duplicate a project under a fresh id and a "(copy)" name.
pub fn duplicate_project(id: &str) -> Result<Project> {
    let mut copy = load_project(id)?;
    copy.id = uuid::Uuid::new_v4().to_string();
    copy.name = format!("{} (copy)", copy.name);
    save_project(&copy)?;
    Ok(copy)
}

fn settings_path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

/// Load settings, or defaults if no settings file exists yet.
pub fn load_settings() -> Result<Settings> {
    let path = settings_path(&config_dir()?);
    if !path.exists() {
        return Ok(Settings::default());
    }
    let bytes = fs::read(&path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Persist settings atomically, stamping the current schema version.
pub fn save_settings(settings: &Settings) -> Result<()> {
    let mut to_write = settings.clone();
    to_write.schema_version = SCHEMA_VERSION;
    let bytes = serde_json::to_vec_pretty(&to_write)?;
    write_atomic(&settings_path(&config_dir()?), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("steampipe-test-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = temp_dir("roundtrip");
        let mut project = Project {
            id: "p1".to_string(),
            name: "Demo".to_string(),
            app_id: "480".to_string(),
            build_counter: 7,
            ..Default::default()
        };
        project.depots.push(crate::model::Depot {
            depot_id: "481".to_string(),
            name: "base".to_string(),
            recursive: true,
            ..Default::default()
        });

        save_project_in(&dir, &project).expect("save");
        let loaded = load_project_in(&dir, "p1").expect("load");

        assert_eq!(loaded.name, "Demo");
        assert_eq!(loaded.app_id, "480");
        assert_eq!(loaded.build_counter, 7);
        assert_eq!(loaded.depots.len(), 1);
        assert_eq!(loaded.depots[0].depot_id, "481");

        let listed = list_projects_in(&dir).expect("list");
        assert_eq!(listed.len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn loads_file_missing_newer_fields() {
        let dir = temp_dir("legacy");
        // An old file: no schema_version, and the project omits several fields
        // added later (depots, build_counter, ignored_findings, ...).
        let legacy = r#"{ "project": { "id": "old", "name": "Legacy", "app_id": "10" } }"#;
        fs::write(project_path(&dir, "old"), legacy).expect("write legacy");

        let loaded = load_project_in(&dir, "old").expect("load legacy");
        assert_eq!(loaded.name, "Legacy");
        assert_eq!(loaded.app_id, "10");
        // Missing fields fall back to Default.
        assert_eq!(loaded.build_counter, 0);
        assert!(loaded.depots.is_empty());
        assert!(loaded.ignored_findings.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ignores_unknown_fields() {
        let dir = temp_dir("unknown");
        let data = r#"{ "schema_version": 1, "project": { "id": "u", "name": "U",
            "surprise_field": 123, "another": ["x"] } }"#;
        fs::write(project_path(&dir, "u"), data).expect("write");
        let loaded = load_project_in(&dir, "u").expect("load");
        assert_eq!(loaded.name, "U");
        fs::remove_dir_all(&dir).ok();
    }
}
