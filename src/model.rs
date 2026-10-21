//! Core domain model: projects, depots, branches, and build records.
//!
//! Every type is `serde`-serializable with a tolerant representation:
//! `#[serde(default)]` fills missing fields from `Default`, and unknown fields
//! are ignored, so files written by newer or older versions still load.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Release stage a branch maps to on Steam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BranchLevel {
    #[default]
    Dev,
    Beta,
    Live,
}

/// Outcome of a build run. Constructed by the history/runner steps (K).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BuildStatus {
    #[default]
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
}

/// A single depot within a project.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Depot {
    /// Steam depot id. Stored as text; validated numeric.
    pub depot_id: String,
    pub name: String,
    pub local_path: PathBuf,
    /// Path mapping inside the depot (the VDF `path` / install root).
    pub depot_path: String,
    pub recursive: bool,
    pub exclusions: Vec<String>,
}

/// A named branch and the release level it targets.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Branch {
    pub name: String,
    pub level: BranchLevel,
}

/// A single pipeline step. Intentionally empty for now (defined in a later step).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineStep {}

/// A game project: everything needed to generate VDFs and run a build.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub id: String,
    pub name: String,
    /// Steam app id. Stored as text; validated numeric.
    pub app_id: String,
    pub content_root: PathBuf,
    pub build_output_dir: PathBuf,
    pub steamcmd_path: PathBuf,
    pub key_art_path: PathBuf,
    pub steam_username: String,
    pub default_account: String,
    pub depots: Vec<Depot>,
    pub branches: Vec<Branch>,
    pub set_live_branch: String,
    pub desc_template: String,
    pub pipeline: Vec<PipelineStep>,
    pub ignore_patterns: Vec<String>,
    pub build_counter: u32,
    /// Release-checklist item labels. Placeholder `Vec<String>` until the
    /// checklist feature defines a richer type.
    pub checklist: Vec<String>,
    /// Secret-scanner finding ids the user chose to ignore.
    pub ignored_findings: Vec<String>,
}

/// One recorded build run. Constructed by the history/runner steps (K).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildRecord {
    pub id: String,
    pub project_id: String,
    /// Unix timestamp in milliseconds. `i64` keeps the type `Default`-able.
    pub timestamp: i64,
    pub description: String,
    pub branch: String,
    pub status: BuildStatus,
    pub duration_ms: u64,
    pub log_path: PathBuf,
    pub git_commit: Option<String>,
    pub manifest_path: Option<PathBuf>,
    pub notes: Option<String>,
}
