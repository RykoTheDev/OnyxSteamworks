#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

use crate::model::{BuildRecord, BuildStatus, Project};

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("could not determine the data directory: {0}")]
    NoDir(String),
}

type Result<T> = std::result::Result<T, HistoryError>;

fn db_path() -> Result<PathBuf> {
    let dir = crate::config::data_dir().map_err(|e| HistoryError::NoDir(e.to_string()))?;
    Ok(dir.join("history.db"))
}

fn status_text(status: BuildStatus) -> &'static str {
    match status {
        BuildStatus::Pending => "pending",
        BuildStatus::Running => "running",
        BuildStatus::Success => "success",
        BuildStatus::Failed => "failed",
        BuildStatus::Cancelled => "cancelled",
    }
}

fn status_from(text: &str) -> BuildStatus {
    match text {
        "success" => BuildStatus::Success,
        "failed" => BuildStatus::Failed,
        "cancelled" => BuildStatus::Cancelled,
        "running" => BuildStatus::Running,
        _ => BuildStatus::Pending,
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn connect() -> Result<Connection> {
    let conn = Connection::open(db_path()?)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS builds (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            description TEXT NOT NULL,
            branch TEXT NOT NULL,
            status TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            log_path TEXT NOT NULL,
            settings_json TEXT NOT NULL,
            git_commit TEXT,
            manifest_path TEXT,
            notes TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_builds_timestamp ON builds(timestamp);",
    )?;
    Ok(conn)
}

pub fn insert(conn: &Connection, record: &BuildRecord) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO builds
            (id, project_id, timestamp, description, branch, status, duration_ms,
             log_path, settings_json, git_commit, manifest_path, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            record.id,
            record.project_id,
            record.timestamp,
            record.description,
            record.branch,
            status_text(record.status),
            record.duration_ms as i64,
            record.log_path.to_string_lossy(),
            record.settings_json,
            record.git_commit,
            record
                .manifest_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            record.notes,
        ],
    )?;
    Ok(())
}

fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<BuildRecord> {
    Ok(BuildRecord {
        id: row.get(0)?,
        project_id: row.get(1)?,
        timestamp: row.get(2)?,
        description: row.get(3)?,
        branch: row.get(4)?,
        status: status_from(&row.get::<_, String>(5)?),
        duration_ms: row.get::<_, i64>(6)? as u64,
        log_path: PathBuf::from(row.get::<_, String>(7)?),
        settings_json: row.get(8)?,
        git_commit: row.get(9)?,
        manifest_path: row.get::<_, Option<String>>(10)?.map(PathBuf::from),
        notes: row.get(11)?,
    })
}

pub fn all(conn: &Connection) -> Result<Vec<BuildRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, timestamp, description, branch, status, duration_ms,
                log_path, settings_json, git_commit, manifest_path, notes
         FROM builds ORDER BY timestamp DESC",
    )?;
    let rows = stmt.query_map([], row_to_record)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM builds WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn make_record(
    project: &Project,
    branch: &str,
    description: &str,
    status: BuildStatus,
    duration_ms: u64,
    log_path: &Path,
) -> BuildRecord {
    BuildRecord {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project.id.clone(),
        timestamp: now_ms(),
        description: description.to_string(),
        branch: branch.to_string(),
        status,
        duration_ms,
        log_path: log_path.to_path_buf(),
        settings_json: serde_json::to_string(project).unwrap_or_default(),
        git_commit: None,
        manifest_path: None,
        notes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, ts: i64, status: BuildStatus) -> BuildRecord {
        BuildRecord {
            id: id.to_string(),
            project_id: "p1".to_string(),
            timestamp: ts,
            description: "nightly".to_string(),
            branch: "beta".to_string(),
            status,
            duration_ms: 1234,
            log_path: PathBuf::from("/tmp/x.log"),
            settings_json: "{}".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn insert_list_delete_roundtrip() {
        let conn = Connection::open_in_memory().expect("mem db");
        conn.execute_batch(
            "CREATE TABLE builds (id TEXT PRIMARY KEY, project_id TEXT, timestamp INTEGER,
             description TEXT, branch TEXT, status TEXT, duration_ms INTEGER, log_path TEXT,
             settings_json TEXT, git_commit TEXT, manifest_path TEXT, notes TEXT);",
        )
        .expect("schema");
        insert(&conn, &record("a", 100, BuildStatus::Success)).expect("insert a");
        insert(&conn, &record("b", 200, BuildStatus::Failed)).expect("insert b");
        let rows = all(&conn).expect("all");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "b");
        assert_eq!(rows[0].status, BuildStatus::Failed);
        delete(&conn, "b").expect("delete");
        let rows = all(&conn).expect("all2");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "a");
    }
}
