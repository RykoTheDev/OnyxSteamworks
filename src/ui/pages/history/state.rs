use std::time::{SystemTime, UNIX_EPOCH};

use crate::history;
use crate::model::{BuildRecord, BuildStatus};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    #[default]
    All,
    Success,
    Failed,
    Cancelled,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum RangeFilter {
    #[default]
    All,
    Day,
    Week,
    Month,
}

pub struct HistoryState {
    pub records: Vec<BuildRecord>,
    pub loaded: bool,
    pub project_filter: usize,
    pub status_filter: StatusFilter,
    pub range_filter: RangeFilter,
    pub detail: Option<BuildRecord>,
    pub detail_log: String,
    pub error: Option<String>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl HistoryState {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            loaded: false,
            project_filter: 0,
            status_filter: StatusFilter::All,
            range_filter: RangeFilter::All,
            detail: None,
            detail_log: String::new(),
            error: None,
        }
    }

    pub fn ensure_loaded(&mut self) {
        if !self.loaded {
            self.reload();
        }
    }

    pub fn reload(&mut self) {
        match history::connect().and_then(|c| history::all(&c)) {
            Ok(rows) => {
                self.records = rows;
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        self.loaded = true;
    }

    pub fn save(&mut self, record: BuildRecord) {
        match history::connect() {
            Ok(conn) => {
                if let Err(e) = history::insert(&conn, &record) {
                    self.error = Some(e.to_string());
                }
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        self.reload();
    }

    pub fn delete(&mut self, id: &str) {
        if let Ok(conn) = history::connect() {
            let _ = history::delete(&conn, id);
        }
        self.reload();
    }

    pub fn latest_for(&self, project_id: &str) -> Option<&BuildRecord> {
        self.records.iter().find(|r| r.project_id == project_id)
    }

    pub fn last_commit_for(&self, project_id: &str) -> Option<String> {
        self.records
            .iter()
            .find(|r| {
                r.project_id == project_id
                    && r.status == BuildStatus::Success
                    && r.git_commit.is_some()
            })
            .and_then(|r| r.git_commit.clone())
    }

    pub fn for_project<'a>(&'a self, project_id: &str) -> Vec<&'a BuildRecord> {
        self.records
            .iter()
            .filter(|r| r.project_id == project_id)
            .collect()
    }

    pub fn last_manifest(&self, project_id: &str, branch: &str) -> Option<std::path::PathBuf> {
        self.records
            .iter()
            .find(|r| {
                r.project_id == project_id
                    && r.branch == branch
                    && r.status == crate::model::BuildStatus::Success
                    && r.manifest_path.is_some()
            })
            .and_then(|r| r.manifest_path.clone())
    }

    pub fn filtered<'a>(&'a self, project_ids: &[String]) -> Vec<&'a BuildRecord> {
        let now = now_ms();
        let window = match self.range_filter {
            RangeFilter::All => None,
            RangeFilter::Day => Some(86_400_000i64),
            RangeFilter::Week => Some(7 * 86_400_000),
            RangeFilter::Month => Some(30 * 86_400_000),
        };
        let want_status = match self.status_filter {
            StatusFilter::All => None,
            StatusFilter::Success => Some(BuildStatus::Success),
            StatusFilter::Failed => Some(BuildStatus::Failed),
            StatusFilter::Cancelled => Some(BuildStatus::Cancelled),
        };
        let want_project = self
            .project_filter
            .checked_sub(1)
            .and_then(|i| project_ids.get(i));
        self.records
            .iter()
            .filter(|r| want_project.is_none_or(|pid| &r.project_id == pid))
            .filter(|r| want_status.is_none_or(|s| r.status == s))
            .filter(|r| window.is_none_or(|w| now - r.timestamp <= w))
            .collect()
    }

    pub fn open_detail(&mut self, record: BuildRecord) {
        self.detail_log = std::fs::read_to_string(&record.log_path).unwrap_or_default();
        self.detail = Some(record);
    }

    pub fn close_detail(&mut self) {
        self.detail = None;
        self.detail_log.clear();
    }
}
