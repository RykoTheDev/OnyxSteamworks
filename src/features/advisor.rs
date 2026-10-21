use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::features::manifest::{self, Manifest};
use crate::model::{BuildRecord, BuildStatus, Project};
use crate::validate::human_size;

const SAMPLE_BYTES: usize = 64 * 1024;
const ENTROPY_FLAG: f32 = 7.9;

#[derive(Debug, Clone)]
pub struct LargeChurn {
    pub path: String,
    pub depot_id: String,
    pub size: u64,
    pub changes: usize,
    pub builds: usize,
}

#[derive(Debug, Clone)]
pub struct EntropyFlag {
    pub path: String,
    pub depot_id: String,
    pub size: u64,
    pub entropy: f32,
}

#[derive(Debug, Clone)]
pub struct Duplicate {
    pub size: u64,
    pub count: usize,
    pub wasted: u64,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AdvisorReport {
    pub large_churn: Vec<LargeChurn>,
    pub entropy: Vec<EntropyFlag>,
    pub duplicates: Vec<Duplicate>,
    pub advice: Vec<String>,
    pub wasted_total: u64,
}

#[derive(Debug, Clone)]
pub struct BuildPoint {
    pub size: u64,
    pub duration_ms: u64,
    pub success: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Analytics {
    pub points: Vec<BuildPoint>,
    pub success_rate: f32,
    pub total_builds: usize,
    pub growth: Option<f32>,
    pub report: AdvisorReport,
}

fn entropy(bytes: &[u8]) -> f32 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0u64; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let len = bytes.len() as f64;
    let mut h = 0.0f64;
    for &c in counts.iter() {
        if c > 0 {
            let p = c as f64 / len;
            h -= p * p.log2();
        }
    }
    h as f32
}

fn sample_entropy(path: &Path) -> Option<f32> {
    let mut handle = std::fs::File::open(path).ok()?.take(SAMPLE_BYTES as u64);
    let mut data = Vec::with_capacity(SAMPLE_BYTES);
    handle.read_to_end(&mut data).ok()?;
    if data.is_empty() {
        return None;
    }
    Some(entropy(&data))
}

fn churn(manifests: &[Manifest], large_bytes: u64) -> Vec<LargeChurn> {
    let n = manifests.len();
    let mut seq: HashMap<(String, String), (Vec<String>, u64)> = HashMap::new();
    for m in manifests {
        for d in &m.depots {
            for f in &d.files {
                let e = seq.entry((d.depot_id.clone(), f.path.clone())).or_default();
                e.0.push(f.hash.clone());
                e.1 = f.size;
            }
        }
    }
    let mut out = Vec::new();
    for ((depot_id, path), (hashes, size)) in seq {
        if size < large_bytes {
            continue;
        }
        let changes = hashes.windows(2).filter(|w| w[0] != w[1]).count();
        if changes * 2 > n {
            out.push(LargeChurn {
                path,
                depot_id,
                size,
                changes,
                builds: n,
            });
        }
    }
    out.sort_by_key(|x| std::cmp::Reverse(x.size));
    out.truncate(20);
    out
}

fn entropy_flags(project: &Project, latest: &Manifest, large_bytes: u64) -> Vec<EntropyFlag> {
    let mut out = Vec::new();
    for d in &latest.depots {
        let root = project
            .depots
            .iter()
            .find(|p| p.depot_id == d.depot_id)
            .map(|p| p.local_path.clone());
        let Some(root) = root else {
            continue;
        };
        for f in &d.files {
            if f.size < large_bytes {
                continue;
            }
            if let Some(h) = sample_entropy(&root.join(&f.path))
                && h > ENTROPY_FLAG
            {
                out.push(EntropyFlag {
                    path: f.path.clone(),
                    depot_id: d.depot_id.clone(),
                    size: f.size,
                    entropy: h,
                });
            }
        }
    }
    out.sort_by_key(|x| std::cmp::Reverse(x.size));
    out.truncate(20);
    out
}

fn duplicates(latest: &Manifest) -> (Vec<Duplicate>, u64) {
    let mut by_hash: HashMap<String, (u64, Vec<String>)> = HashMap::new();
    for d in &latest.depots {
        for f in &d.files {
            if f.hash.is_empty() {
                continue;
            }
            let e = by_hash
                .entry(f.hash.clone())
                .or_insert((f.size, Vec::new()));
            e.1.push(format!("{}:{}", d.depot_id, f.path));
        }
    }
    let mut out = Vec::new();
    let mut wasted_total = 0u64;
    for (_, (size, paths)) in by_hash {
        if paths.len() > 1 {
            let wasted = size * (paths.len() as u64 - 1);
            wasted_total += wasted;
            out.push(Duplicate {
                size,
                count: paths.len(),
                wasted,
                paths,
            });
        }
    }
    out.sort_by_key(|x| std::cmp::Reverse(x.wasted));
    out.truncate(20);
    (out, wasted_total)
}

pub fn advise(project: &Project, manifests: &[Manifest], large_bytes: u64) -> AdvisorReport {
    let large_churn = churn(manifests, large_bytes);
    let (entropy, duplicates, wasted_total) = match manifests.last() {
        Some(latest) => {
            let (dups, wasted) = duplicates(latest);
            (entropy_flags(project, latest, large_bytes), dups, wasted)
        }
        None => (Vec::new(), Vec::new(), 0),
    };

    let mut advice = Vec::new();
    if !large_churn.is_empty() {
        advice.push("Large assets that change in most builds may upload faster in their own depot, so Steam can skip the depots that did not change.".to_string());
    }
    if !entropy.is_empty() {
        advice.push("Some large files already look compressed or encrypted, so they diff poorly; avoiding re-packing or recompressing these archives every build may keep uploads small.".to_string());
    }
    if !duplicates.is_empty() {
        advice.push(format!(
            "Duplicate files across depots waste about {}; keeping a single copy may help.",
            human_size(wasted_total)
        ));
    }

    AdvisorReport {
        large_churn,
        entropy,
        duplicates,
        advice,
        wasted_total,
    }
}

pub fn run(project: &Project, records: &[BuildRecord], large_bytes: u64, keep: usize) -> Analytics {
    let total_builds = records.len();
    let successes = records
        .iter()
        .filter(|r| r.status == BuildStatus::Success)
        .count();
    let success_rate = if total_builds == 0 {
        0.0
    } else {
        successes as f32 / total_builds as f32
    };

    let mut chrono: Vec<&BuildRecord> = records.iter().collect();
    chrono.sort_by_key(|r| r.timestamp);

    let mut points = Vec::new();
    let mut manifests = Vec::new();
    for rec in &chrono {
        let size = rec
            .manifest_path
            .as_ref()
            .and_then(|p| manifest::load_manifest(p).ok())
            .map(|m| {
                let total: u64 = m
                    .depots
                    .iter()
                    .flat_map(|d| d.files.iter())
                    .map(|f| f.size)
                    .sum();
                manifests.push(m);
                total
            });
        if let Some(size) = size {
            points.push(BuildPoint {
                size,
                duration_ms: rec.duration_ms,
                success: rec.status == BuildStatus::Success,
            });
        }
    }
    if manifests.len() > keep {
        manifests.drain(0..manifests.len() - keep);
    }

    let growth = if points.len() >= 2 {
        let a = points[points.len() - 2].size.max(1) as f32;
        let b = points[points.len() - 1].size as f32;
        Some(b / a - 1.0)
    } else {
        None
    };

    let report = advise(project, &manifests, large_bytes);
    Analytics {
        points,
        success_rate,
        total_builds,
        growth,
        report,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::manifest::{DepotManifest, FileEntry};

    fn mf(files: &[(&str, &str, u64)]) -> Manifest {
        Manifest {
            created_ms: 0,
            depots: vec![DepotManifest {
                depot_id: "1".into(),
                files: files
                    .iter()
                    .map(|(p, h, s)| FileEntry {
                        path: (*p).into(),
                        hash: (*h).into(),
                        size: *s,
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn entropy_of_uniform_is_low_and_random_is_high() {
        let zeros = [0u8; 1024];
        assert!(entropy(&zeros) < 0.01);
        let spread: Vec<u8> = (0..=255u16).cycle().take(2048).map(|v| v as u8).collect();
        assert!(entropy(&spread) > 7.5);
    }

    #[test]
    fn churn_flags_large_frequently_changing_files() {
        let big = 200 * 1024 * 1024;
        let manifests = vec![
            mf(&[("a.pak", "h1", big), ("b.txt", "t1", 10)]),
            mf(&[("a.pak", "h2", big), ("b.txt", "t1", 10)]),
            mf(&[("a.pak", "h3", big), ("b.txt", "t1", 10)]),
        ];
        let flagged = churn(&manifests, 100 * 1024 * 1024);
        assert_eq!(flagged.len(), 1);
        assert_eq!(flagged[0].path, "a.pak");
        assert_eq!(flagged[0].changes, 2);
    }

    #[test]
    fn duplicates_sum_wasted_bytes() {
        let latest = Manifest {
            created_ms: 0,
            depots: vec![
                DepotManifest {
                    depot_id: "1".into(),
                    files: vec![FileEntry {
                        path: "a".into(),
                        hash: "same".into(),
                        size: 100,
                    }],
                },
                DepotManifest {
                    depot_id: "2".into(),
                    files: vec![FileEntry {
                        path: "b".into(),
                        hash: "same".into(),
                        size: 100,
                    }],
                },
            ],
        };
        let (dups, wasted) = duplicates(&latest);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].count, 2);
        assert_eq!(wasted, 100);
    }
}
