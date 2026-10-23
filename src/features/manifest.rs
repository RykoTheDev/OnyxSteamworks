use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::model::Project;

mod diff;
pub use diff::{DiffEntry, DiffStatus, DiffSummary, diff};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DepotManifest {
    pub depot_id: String,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub created_ms: i64,
    pub depots: Vec<DepotManifest>,
}

#[derive(Default)]
pub struct Progress {
    pub total: AtomicUsize,
    pub done: AtomicUsize,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("manifest io: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest encode: {0}")]
    Json(#[from] serde_json::Error),
    #[error("data directory is unavailable")]
    DataDir,
}

#[derive(Default, Serialize, Deserialize)]
struct CacheEntry {
    size: u64,
    mtime: i64,
    hash: String,
}

type Cache = HashMap<String, CacheEntry>;

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

fn exclusion_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
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

struct Candidate {
    depot_idx: usize,
    abs: PathBuf,
    rel: String,
    size: u64,
    mtime: i64,
}

fn collect(project: &Project) -> Vec<Candidate> {
    let mut out = Vec::new();
    for (di, depot) in project.depots.iter().enumerate() {
        if depot.local_path.as_os_str().is_empty() {
            continue;
        }
        let mut patterns = depot.exclusions.clone();
        patterns.extend(project.ignore_patterns.iter().cloned());
        let excluded = exclusion_set(&patterns);
        let depth = if depot.recursive { usize::MAX } else { 1 };
        for entry in WalkDir::new(&depot.local_path)
            .max_depth(depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy();
            if excluded.is_match(name.as_ref()) {
                continue;
            }
            let Ok(md) = entry.metadata() else {
                continue;
            };
            let rel = entry
                .path()
                .strip_prefix(&depot.local_path)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            let mtime = md
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            out.push(Candidate {
                depot_idx: di,
                abs: entry.path().to_path_buf(),
                rel,
                size: md.len(),
                mtime,
            });
        }
    }
    out
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize().to_hex().to_string())
}

fn cache_path() -> Option<PathBuf> {
    crate::config::data_dir()
        .ok()
        .map(|d| d.join("hash_cache.json"))
}

fn load_cache() -> Cache {
    cache_path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_cache(cache: &Cache) {
    if let Some(path) = cache_path()
        && let Ok(bytes) = serde_json::to_vec(cache)
    {
        let _ = std::fs::write(path, bytes);
    }
}

pub fn build_manifest(project: &Project, progress: Option<&Progress>) -> Manifest {
    let candidates = collect(project);
    if let Some(p) = progress {
        p.total.store(candidates.len(), Ordering::Relaxed);
        p.done.store(0, Ordering::Relaxed);
    }
    let cache = load_cache();
    let done = AtomicUsize::new(0);

    let hashed: Vec<(usize, FileEntry, CacheEntry)> = candidates
        .par_iter()
        .map(|c| {
            let key = c.abs.to_string_lossy().to_string();
            let hash = match cache.get(&key) {
                Some(e) if e.size == c.size && e.mtime == c.mtime => e.hash.clone(),
                _ => hash_file(&c.abs).unwrap_or_default(),
            };
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(p) = progress {
                p.done.store(n, Ordering::Relaxed);
            }
            let entry = FileEntry {
                path: c.rel.clone(),
                size: c.size,
                hash: hash.clone(),
            };
            (
                c.depot_idx,
                entry,
                CacheEntry {
                    size: c.size,
                    mtime: c.mtime,
                    hash,
                },
            )
        })
        .collect();

    let mut new_cache = Cache::new();
    let mut grouped: Vec<Vec<FileEntry>> = vec![Vec::new(); project.depots.len()];
    for (di, entry, ce) in hashed {
        if let Some(c) = candidates
            .iter()
            .find(|c| c.rel == entry.path && c.depot_idx == di)
        {
            new_cache.insert(c.abs.to_string_lossy().to_string(), ce);
        }
        if di < grouped.len() {
            grouped[di].push(entry);
        }
    }
    save_cache(&new_cache);

    let depots = project
        .depots
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let mut files = std::mem::take(&mut grouped[i]);
            files.sort_by(|a, b| a.path.cmp(&b.path));
            DepotManifest {
                depot_id: d.depot_id.clone(),
                files,
            }
        })
        .collect();

    Manifest {
        created_ms: now_ms(),
        depots,
    }
}

pub fn save_manifest(manifest: &Manifest) -> Result<PathBuf, ManifestError> {
    let dir = crate::config::data_dir()
        .map_err(|_| ManifestError::DataDir)?
        .join("manifests");
    let path = dir.join(format!("{}.mf.gz", uuid::Uuid::new_v4()));
    save_manifest_to(manifest, &path)?;
    Ok(path)
}

pub fn save_manifest_to(manifest: &Manifest, path: &Path) -> Result<(), ManifestError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec(manifest)?;
    let tmp = path.with_extension("tmp");
    let file = std::fs::File::create(&tmp)?;
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    encoder.write_all(&json)?;
    encoder.finish()?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load_manifest(path: &Path) -> Result<Manifest, ManifestError> {
    let file = std::fs::File::open(path)?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Depot;

    #[test]
    fn build_manifest_walks_depot_and_hashes_files() {
        let dir = std::env::temp_dir().join(format!("onyx_bm_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, body) in [("a.txt", "hello"), ("b.txt", "world")] {
            let mut f = std::fs::File::create(dir.join(name)).unwrap();
            f.write_all(body.as_bytes()).unwrap();
        }
        let project = Project {
            depots: vec![Depot {
                depot_id: "1001".into(),
                local_path: dir.clone(),
                recursive: true,
                exclusions: vec!["*.skip".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let manifest = build_manifest(&project, None);
        assert_eq!(manifest.depots.len(), 1);
        let files = &manifest.depots[0].files;
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "a.txt");
        assert_eq!(files[0].size, 5);
        assert!(!files[0].hash.is_empty());
        assert_ne!(files[0].hash, files[1].hash);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let manifest = Manifest {
            created_ms: 42,
            depots: vec![DepotManifest {
                depot_id: "1001".into(),
                files: vec![FileEntry {
                    path: "a.txt".into(),
                    size: 3,
                    hash: "deadbeef".into(),
                }],
            }],
        };
        let path = std::env::temp_dir().join(format!("onyx_mf_{}.mf.gz", uuid::Uuid::new_v4()));
        let file = std::fs::File::create(&path).expect("create");
        let mut enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        enc.write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        enc.finish().unwrap();
        let loaded = load_manifest(&path).expect("load");
        assert_eq!(loaded, manifest);
    }
}
