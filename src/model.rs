use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BranchLevel {
    #[default]
    Dev,
    Beta,
    Live,
}

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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Depot {
    pub depot_id: String,
    pub name: String,
    pub local_path: PathBuf,
    pub depot_path: String,
    pub recursive: bool,
    pub exclusions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Branch {
    pub name: String,
    pub level: BranchLevel,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StepKind {
    #[default]
    Upload,
    RunCommand,
    Clean,
    Backup,
    Notify,
    Confirm,
    BuildGame,
}

impl StepKind {
    pub const ALL: [StepKind; 7] = [
        StepKind::BuildGame,
        StepKind::Upload,
        StepKind::RunCommand,
        StepKind::Clean,
        StepKind::Backup,
        StepKind::Notify,
        StepKind::Confirm,
    ];

    pub fn label(self) -> &'static str {
        match self {
            StepKind::BuildGame => "Build game",
            StepKind::Upload => "Upload to Steam",
            StepKind::RunCommand => "Run command",
            StepKind::Clean => "Clean",
            StepKind::Backup => "Backup",
            StepKind::Notify => "Notify",
            StepKind::Confirm => "Wait for approval",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RunIf {
    #[default]
    Always,
    PreviousFailed,
    FilesChanged,
}

impl RunIf {
    pub const ALL: [RunIf; 3] = [RunIf::Always, RunIf::PreviousFailed, RunIf::FilesChanged];

    pub fn label(self) -> &'static str {
        match self {
            RunIf::Always => "Always",
            RunIf::PreviousFailed => "Only if an earlier step failed",
            RunIf::FilesChanged => "Only if files changed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineStep {
    pub id: String,
    pub name: String,
    pub kind: StepKind,
    pub enabled: bool,
    pub continue_on_failure: bool,
    pub timeout_secs: u32,
    pub exe: String,
    pub args: String,
    pub working_dir: String,
    pub env: String,
    pub targets: Vec<String>,
    pub message: String,
    pub engine: String,
    pub platform: String,
    pub output: String,
    pub export_preset: String,
    pub run_if: RunIf,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub steps: Vec<PipelineStep>,
}

impl Pipeline {
    pub fn new(name: &str, steps: Vec<PipelineStep>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            steps,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RunOutcome {
    #[default]
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StepOutcome {
    #[default]
    NotRun,
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StepRecord {
    pub name: String,
    pub kind: StepKind,
    pub outcome: StepOutcome,
    pub duration_ms: u64,
    pub log: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineRun {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub started_ms: i64,
    pub duration_ms: u64,
    pub outcome: RunOutcome,
    pub steps: Vec<StepRecord>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub app_id: String,
    pub content_root: PathBuf,
    pub build_output_dir: PathBuf,
    pub steamcmd_path: PathBuf,
    pub repo_path: PathBuf,
    pub steam_username: String,
    pub default_account: String,
    pub depots: Vec<Depot>,
    pub branches: Vec<Branch>,
    pub set_live_branch: String,
    pub desc_template: String,
    pub version_file: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pipeline: Vec<PipelineStep>,
    pub pipelines: Vec<Pipeline>,
    pub ignore_patterns: Vec<String>,
    pub build_counter: u32,
    pub checklist: Vec<ChecklistItem>,
    pub ignored_findings: Vec<String>,
}

impl Project {
    pub fn migrate(&mut self) {
        if !self.pipeline.is_empty() {
            let steps = std::mem::take(&mut self.pipeline);
            if self.pipelines.is_empty() {
                self.pipelines.push(Pipeline::new("Default", steps));
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildRecord {
    pub id: String,
    pub project_id: String,
    pub timestamp: i64,
    pub description: String,
    pub branch: String,
    pub status: BuildStatus,
    pub duration_ms: u64,
    pub log_path: PathBuf,
    pub settings_json: String,
    pub git_commit: Option<String>,
    pub manifest_path: Option<PathBuf>,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_pipeline_moves_into_a_default_named_pipeline() {
        let json =
            r#"{"name":"Game","pipeline":[{"name":"Upload","kind":"Upload","enabled":true}]}"#;
        let mut project: Project = serde_json::from_str(json).expect("parse");
        project.migrate();
        assert!(project.pipeline.is_empty());
        assert_eq!(project.pipelines.len(), 1);
        assert_eq!(project.pipelines[0].name, "Default");
        assert_eq!(project.pipelines[0].steps[0].name, "Upload");
        let saved = serde_json::to_string(&project).expect("encode");
        assert!(!saved.contains("\"pipeline\":"));
    }

    #[test]
    fn migrate_keeps_existing_named_pipelines() {
        let mut project = Project {
            pipelines: vec![Pipeline::new("Nightly", Vec::new())],
            ..Default::default()
        };
        project.migrate();
        assert_eq!(project.pipelines.len(), 1);
        assert_eq!(project.pipelines[0].name, "Nightly");
    }
}
