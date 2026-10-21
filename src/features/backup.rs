use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::model::Project;
use crate::validate::human_size;

const MARGIN: u64 = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("the content root does not exist")]
    NoContentRoot,
    #[error("no backup folder is set")]
    NoBackupDir,
    #[error("not enough free disk space: need about {need}, have {have} free")]
    LowDisk { need: String, have: String },
    #[error("backup io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("metadata error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Default)]
pub struct Progress {
    pub done: AtomicU64,
    pub total: AtomicU64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupMeta {
    pub project_id: String,
    pub project_name: String,
    pub build_number: u32,
    pub created_ms: i64,
}

#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub meta: BackupMeta,
    pub size: u64,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn normalize(pattern: &str) -> String {
    if pattern.starts_with('.') && !pattern.contains('*') && !pattern.contains('/') {
        format!("*{pattern}")
    } else {
        pattern.to_string()
    }
}

fn exclusion_set(project: &Project) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    let patterns = project
        .depots
        .iter()
        .flat_map(|d| d.exclusions.iter())
        .chain(project.ignore_patterns.iter());
    for pat in patterns {
        let pat = pat.trim();
        if pat.is_empty() {
            continue;
        }
        if let Ok(glob) = Glob::new(&normalize(pat)) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

pub fn project_dir(backup_root: &Path, project_id: &str) -> PathBuf {
    backup_root.join(project_id)
}

pub fn next_build_number(backup_root: &Path, project_id: &str) -> u32 {
    list_backups(backup_root, project_id)
        .iter()
        .map(|b| b.meta.build_number)
        .max()
        .map(|n| n + 1)
        .unwrap_or(1)
}

pub fn create_backup(
    project: &Project,
    backup_root: &Path,
    build_number: u32,
    keep: u32,
    auto_prune: bool,
    progress: Option<&Progress>,
) -> Result<BackupInfo, BackupError> {
    if backup_root.as_os_str().is_empty() {
        return Err(BackupError::NoBackupDir);
    }
    let content = &project.content_root;
    if content.as_os_str().is_empty() || !content.is_dir() {
        return Err(BackupError::NoContentRoot);
    }

    let excluded = exclusion_set(project);
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut total_bytes = 0u64;
    for entry in WalkDir::new(content).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if excluded.is_match(name.as_ref()) {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(content)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        total_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        files.push((entry.path().to_path_buf(), rel));
    }
    if let Some(p) = progress {
        p.total.store(total_bytes.max(1), Ordering::Relaxed);
        p.done.store(0, Ordering::Relaxed);
    }

    let dir = project_dir(backup_root, &project.id);
    std::fs::create_dir_all(&dir)?;
    if let Some(free) = crate::features::doctor::sys::free_space(&dir)
        && free < total_bytes + MARGIN
    {
        return Err(BackupError::LowDisk {
            need: human_size(total_bytes + MARGIN),
            have: human_size(free),
        });
    }

    let zip_path = dir.join(format!("{build_number}.zip"));
    let file = File::create(&zip_path)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut buf = vec![0u8; 64 * 1024];
    for (abs, rel) in &files {
        zip.start_file(rel.as_str(), opts)?;
        let mut source = File::open(abs)?;
        loop {
            let n = source.read(&mut buf)?;
            if n == 0 {
                break;
            }
            zip.write_all(&buf[..n])?;
            if let Some(p) = progress {
                p.done.fetch_add(n as u64, Ordering::Relaxed);
            }
        }
    }
    zip.finish()?;

    let meta = BackupMeta {
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        build_number,
        created_ms: now_ms(),
    };
    std::fs::write(zip_path.with_extension("json"), serde_json::to_vec(&meta)?)?;
    if auto_prune {
        prune(&dir, keep);
    }
    let size = std::fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(0);
    Ok(BackupInfo {
        path: zip_path,
        meta,
        size,
    })
}

pub fn list_backups(backup_root: &Path, project_id: &str) -> Vec<BackupInfo> {
    let dir = project_dir(backup_root, project_id);
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("zip") {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let meta = std::fs::read(path.with_extension("json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<BackupMeta>(&b).ok())
            .unwrap_or_default();
        out.push(BackupInfo { path, meta, size });
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.meta.created_ms));
    out
}

pub fn delete_backup(path: &Path) -> Result<(), BackupError> {
    std::fs::remove_file(path)?;
    let _ = std::fs::remove_file(path.with_extension("json"));
    Ok(())
}

pub fn restore_backup(zip_path: &Path, dest: &Path) -> Result<(), BackupError> {
    std::fs::create_dir_all(dest)?;
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(dest)?;
    Ok(())
}

fn prune(dir: &Path, keep: u32) {
    let mut zips: Vec<(PathBuf, i64)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("zip"))
        .map(|p| {
            let ts = std::fs::metadata(&p)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
            (p, ts.map(|d| d.as_millis() as i64).unwrap_or(0))
        })
        .collect();
    zips.sort_by_key(|(_, ts)| std::cmp::Reverse(*ts));
    for (path, _) in zips.into_iter().skip(keep as usize) {
        let _ = delete_backup(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Depot;

    fn write(dir: &Path, name: &str, body: &[u8]) {
        let mut f = File::create(dir.join(name)).unwrap();
        f.write_all(body).unwrap();
    }

    fn tempdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("onyx_bk_{tag}_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn backup_zips_content_and_respects_exclusions() {
        let content = tempdir("content");
        write(&content, "keep.txt", b"hello");
        write(&content, "junk.log", b"noise");
        let root = tempdir("root");
        let project = Project {
            id: "p1".into(),
            name: "Game".into(),
            content_root: content.clone(),
            depots: vec![Depot {
                exclusions: vec!["*.log".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let info = create_backup(&project, &root, 1, 5, false, None).expect("backup");
        assert!(info.path.exists());
        let mut archive = zip::ZipArchive::new(File::open(&info.path).unwrap()).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains(&"keep.txt".to_string()));
        assert!(!names.iter().any(|n| n.ends_with(".log")));
    }

    #[test]
    fn list_and_next_number_track_backups() {
        let content = tempdir("content_list");
        write(&content, "a.txt", b"data");
        let root = tempdir("root_list");
        let project = Project {
            id: "p3".into(),
            name: "Game".into(),
            content_root: content,
            ..Default::default()
        };
        assert_eq!(next_build_number(&root, "p3"), 1);
        create_backup(&project, &root, 1, 5, false, None).expect("backup");
        let listed = list_backups(&root, "p3");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].meta.build_number, 1);
        assert_eq!(listed[0].meta.project_name, "Game");
        assert_eq!(next_build_number(&root, "p3"), 2);
    }

    #[test]
    fn restore_extracts_files() {
        let content = tempdir("content2");
        write(&content, "a.txt", b"data");
        let root = tempdir("root2");
        let project = Project {
            id: "p2".into(),
            content_root: content,
            ..Default::default()
        };
        let info = create_backup(&project, &root, 3, 5, false, None).expect("backup");
        let dest = tempdir("dest");
        restore_backup(&info.path, &dest).expect("restore");
        assert_eq!(std::fs::read_to_string(dest.join("a.txt")).unwrap(), "data");
    }

    #[test]
    fn prune_keeps_last_n() {
        let dir = tempdir("prune");
        for n in 0..5 {
            write(&dir, &format!("{n}.zip"), b"x");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        prune(&dir, 2);
        let remaining = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("zip"))
            .count();
        assert_eq!(remaining, 2);
    }
}
