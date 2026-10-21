use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use walkdir::WalkDir;

use crate::features::manifest::Manifest;

#[derive(Default)]
pub struct Progress {
    pub done: AtomicUsize,
    pub total: AtomicUsize,
}

#[derive(Debug, Clone, Default)]
pub struct VerifyReport {
    pub matched: usize,
    pub missing: Vec<String>,
    pub mismatched: Vec<String>,
    pub extra: Vec<String>,
}

impl VerifyReport {
    pub fn all_match(&self) -> bool {
        self.missing.is_empty() && self.mismatched.is_empty() && self.extra.is_empty()
    }

    pub fn text(&self) -> String {
        let mut out = format!(
            "Verify report\nMatched: {}\nMissing: {}\nMismatched: {}\nExtra: {}\n",
            self.matched,
            self.missing.len(),
            self.mismatched.len(),
            self.extra.len(),
        );
        for (title, list) in [
            ("Missing", &self.missing),
            ("Mismatched", &self.mismatched),
            ("Extra", &self.extra),
        ] {
            if !list.is_empty() {
                out.push_str(&format!("\n{title}:\n"));
                for path in list {
                    out.push_str(&format!("  {path}\n"));
                }
            }
        }
        out
    }
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize().to_hex().to_string())
}

pub fn hash_dir(root: &Path, progress: Option<&Progress>) -> HashMap<String, String> {
    let files: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
    if let Some(p) = progress {
        p.total.store(files.len().max(1), Ordering::Relaxed);
        p.done.store(0, Ordering::Relaxed);
    }
    let mut map = HashMap::new();
    for entry in files {
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        if let Ok(hash) = hash_file(entry.path()) {
            map.insert(rel, hash);
        }
        if let Some(p) = progress {
            p.done.fetch_add(1, Ordering::Relaxed);
        }
    }
    map
}

fn manifest_map(manifest: &Manifest) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for depot in &manifest.depots {
        for file in &depot.files {
            map.insert(file.path.clone(), file.hash.clone());
        }
    }
    map
}

pub fn compare(manifest: &Manifest, downloaded: &HashMap<String, String>) -> VerifyReport {
    let want = manifest_map(manifest);
    let mut report = VerifyReport::default();
    for (path, hash) in &want {
        match downloaded.get(path) {
            None => report.missing.push(path.clone()),
            Some(got) if got != hash => report.mismatched.push(path.clone()),
            Some(_) => report.matched += 1,
        }
    }
    for path in downloaded.keys() {
        if !want.contains_key(path) {
            report.extra.push(path.clone());
        }
    }
    report.missing.sort();
    report.mismatched.sort();
    report.extra.sort();
    report
}

pub fn access_issue(log: &str) -> Option<&'static str> {
    let l = log.to_ascii_lowercase();
    if l.contains("no subscription") || l.contains("no license") {
        Some(
            "The build account doesn't own a license to download this app or branch, so it can't be verified.",
        )
    } else if l.contains("invalid password") || l.contains("password rejected") {
        Some(
            "The login or branch password was rejected. A protected branch needs its password and the account must be signed in.",
        )
    } else if l.contains("two-factor") || l.contains("steam guard") || l.contains("rate limit") {
        Some(
            "The build account needs to sign in with Steam Guard first. Authorize the account, then verify.",
        )
    } else if l.contains("access denied") || l.contains("not have permission") {
        Some(
            "Access was denied. The build account may not have permission to download this app or branch.",
        )
    } else if l.contains("no update available")
        || l.contains("invalid depot")
        || l.contains("not available")
    {
        Some(
            "That build may not be live on this branch yet. Verify can only download a build once it is live.",
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::manifest::{DepotManifest, FileEntry, Manifest};

    fn manifest(files: &[(&str, &str)]) -> Manifest {
        Manifest {
            created_ms: 0,
            depots: vec![DepotManifest {
                depot_id: "1".into(),
                files: files
                    .iter()
                    .map(|(p, h)| FileEntry {
                        path: (*p).into(),
                        size: 1,
                        hash: (*h).into(),
                    })
                    .collect(),
            }],
        }
    }

    fn downloaded(files: &[(&str, &str)]) -> HashMap<String, String> {
        files
            .iter()
            .map(|(p, h)| ((*p).to_string(), (*h).to_string()))
            .collect()
    }

    #[test]
    fn all_match_when_identical() {
        let m = manifest(&[("a", "h1"), ("b", "h2")]);
        let d = downloaded(&[("a", "h1"), ("b", "h2")]);
        let r = compare(&m, &d);
        assert!(r.all_match());
        assert_eq!(r.matched, 2);
    }

    #[test]
    fn detects_missing_mismatch_extra() {
        let m = manifest(&[("keep", "h"), ("gone", "g"), ("changed", "old")]);
        let d = downloaded(&[("keep", "h"), ("changed", "new"), ("fresh", "f")]);
        let r = compare(&m, &d);
        assert_eq!(r.matched, 1);
        assert_eq!(r.missing, vec!["gone".to_string()]);
        assert_eq!(r.mismatched, vec!["changed".to_string()]);
        assert_eq!(r.extra, vec!["fresh".to_string()]);
        assert!(!r.all_match());
    }

    #[test]
    fn access_issue_detects_no_subscription_and_login() {
        assert!(access_issue("ERROR! Failed to install app '480' (No subscription)").is_some());
        assert!(access_issue("FAILED login with result code Invalid Password").is_some());
        assert!(access_issue("Success! App fully installed").is_none());
    }
}
