use std::path::PathBuf;

use crate::features::manifest::{self, Manifest};
use crate::model::{BuildStatus, Project};

fn baseline_path(project: &Project) -> Option<PathBuf> {
    crate::config::data_dir().ok().map(|d| {
        d.join("manifests")
            .join(format!("pipeline-{}.mf.gz", project.id))
    })
}

pub fn record_baseline(project: &Project) -> Result<(), String> {
    let path = baseline_path(project).ok_or("data directory is unavailable")?;
    let current = manifest::build_manifest(project, None);
    manifest::save_manifest_to(&current, &path).map_err(|e| e.to_string())
}

fn last_build_manifest(project: &Project) -> Option<Manifest> {
    let conn = crate::history::connect().ok()?;
    crate::history::all(&conn)
        .ok()?
        .into_iter()
        .find(|r| {
            r.project_id == project.id
                && r.status == BuildStatus::Success
                && r.manifest_path.is_some()
        })
        .and_then(|r| r.manifest_path)
        .and_then(|p| manifest::load_manifest(&p).ok())
}

pub fn newest(a: Option<Manifest>, b: Option<Manifest>) -> Option<Manifest> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if a.created_ms >= b.created_ms { a } else { b }),
        (a, b) => a.or(b),
    }
}

pub fn changed_since(current: &Manifest, baseline: Option<&Manifest>) -> usize {
    match baseline {
        Some(prev) => manifest::diff(current, Some(prev)).changed(),
        None => usize::MAX,
    }
}

pub fn files_changed(project: &Project) -> Result<usize, String> {
    let pipeline_baseline = baseline_path(project)
        .filter(|p| p.is_file())
        .and_then(|p| manifest::load_manifest(&p).ok());
    let baseline = newest(pipeline_baseline, last_build_manifest(project));
    let current = manifest::build_manifest(project, None);
    if current.depots.iter().all(|d| d.files.is_empty()) {
        return Err("the depot folders contain no files".into());
    }
    Ok(changed_since(&current, baseline.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::manifest::{DepotManifest, FileEntry};

    fn manifest(created_ms: i64, hash: &str) -> Manifest {
        Manifest {
            created_ms,
            depots: vec![DepotManifest {
                depot_id: "1".into(),
                files: vec![FileEntry {
                    path: "game.exe".into(),
                    size: 10,
                    hash: hash.into(),
                }],
            }],
        }
    }

    #[test]
    fn picks_the_newest_baseline() {
        let got = newest(Some(manifest(5, "a")), Some(manifest(9, "b"))).unwrap();
        assert_eq!(got.created_ms, 9);
        assert_eq!(newest(None, Some(manifest(1, "a"))).unwrap().created_ms, 1);
        assert!(newest(None, None).is_none());
    }

    #[test]
    fn counts_changes_and_treats_no_baseline_as_changed() {
        let current = manifest(10, "a");
        assert_eq!(changed_since(&current, Some(&manifest(1, "a"))), 0);
        assert_eq!(changed_since(&current, Some(&manifest(1, "b"))), 1);
        assert_eq!(changed_since(&current, None), usize::MAX);
    }
}
