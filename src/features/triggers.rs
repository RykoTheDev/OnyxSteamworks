use std::path::{Path, PathBuf};
use std::process::Command;

use crate::features::schedule::{Schedule, TriggerKind};
use crate::model::Project;

const MINUTE_MS: i64 = 60_000;

pub fn watch_target(s: &Schedule, project: &Project) -> PathBuf {
    let custom = s.watch_path.trim();
    if !custom.is_empty() {
        return PathBuf::from(custom);
    }
    match s.trigger {
        TriggerKind::GitTag => crate::features::patchnotes::repo_path(project),
        _ => crate::features::maintenance::resolve(project).content_root,
    }
}

pub fn folder_fingerprint(dir: &Path) -> Result<String, String> {
    if !dir.is_dir() {
        return Err(format!("{} is not a folder", dir.display()));
    }
    let mut count = 0u64;
    let mut size = 0u64;
    let mut newest = 0u64;
    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        count += 1;
        size += meta.len();
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        newest = newest.max(mtime);
    }
    Ok(format!("{count}:{size}:{newest}"))
}

pub fn newest_tag(repo: &Path, pattern: &str) -> Result<Option<String>, String> {
    let pattern = if pattern.trim().is_empty() {
        "*"
    } else {
        pattern.trim()
    };
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo)
        .args(["tag", "--list", pattern, "--sort=-creatordate"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("git is not available: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string))
}

pub fn observe(s: &Schedule, project: &Project) -> Result<String, String> {
    let target = watch_target(s, project);
    match s.trigger {
        TriggerKind::Time => Ok(String::new()),
        TriggerKind::FolderChange => folder_fingerprint(&target),
        TriggerKind::GitTag => newest_tag(&target, &s.tag_pattern).map(|t| t.unwrap_or_default()),
    }
}

pub fn folder_step(s: &mut Schedule, fingerprint: &str, now: i64) -> bool {
    if s.baseline.is_empty() {
        s.baseline = fingerprint.to_string();
        s.pending.clear();
        return false;
    }
    if fingerprint == s.baseline {
        s.pending.clear();
        return false;
    }
    if fingerprint != s.pending {
        s.pending = fingerprint.to_string();
        s.pending_since_ms = now;
        return false;
    }
    now - s.pending_since_ms >= s.quiet_minutes.max(1) as i64 * MINUTE_MS
}

pub fn tag_step(s: &mut Schedule, newest: &str) -> bool {
    if newest.is_empty() {
        return false;
    }
    if s.baseline.is_empty() {
        s.baseline = newest.to_string();
        return false;
    }
    newest != s.baseline
}

pub fn step(s: &mut Schedule, observed: &str, now: i64) -> bool {
    match s.trigger {
        TriggerKind::Time => false,
        TriggerKind::FolderChange => folder_step(s, observed, now),
        TriggerKind::GitTag => tag_step(s, observed),
    }
}

pub fn fired(s: &mut Schedule, observed: &str) {
    s.last_trigger_note = match s.trigger {
        TriggerKind::GitTag => format!("tag {observed}"),
        _ => "files changed".to_string(),
    };
    if s.trigger == TriggerKind::GitTag {
        s.baseline = observed.to_string();
    }
    s.pending.clear();
}

pub fn rebaseline_after_run(s: &mut Schedule, project: &Project) {
    if s.trigger == TriggerKind::FolderChange
        && let Ok(fp) = folder_fingerprint(&watch_target(s, project))
    {
        s.baseline = fp;
        s.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder() -> Schedule {
        Schedule {
            trigger: TriggerKind::FolderChange,
            quiet_minutes: 10,
            ..Default::default()
        }
    }

    #[test]
    fn folder_waits_for_a_quiet_period_after_changes() {
        let mut s = folder();
        assert!(!folder_step(&mut s, "a", 0));
        assert_eq!(s.baseline, "a");
        assert!(!folder_step(&mut s, "a", MINUTE_MS));
        assert!(!folder_step(&mut s, "b", 2 * MINUTE_MS));
        assert!(!folder_step(&mut s, "c", 5 * MINUTE_MS));
        assert!(!folder_step(&mut s, "c", 14 * MINUTE_MS));
        assert!(folder_step(&mut s, "c", 15 * MINUTE_MS));
    }

    #[test]
    fn folder_change_that_reverts_cancels_the_trigger() {
        let mut s = folder();
        folder_step(&mut s, "a", 0);
        folder_step(&mut s, "b", MINUTE_MS);
        assert!(!folder_step(&mut s, "a", 30 * MINUTE_MS));
        assert!(s.pending.is_empty());
    }

    #[test]
    fn tags_fire_once_per_new_tag() {
        let mut s = Schedule {
            trigger: TriggerKind::GitTag,
            ..Default::default()
        };
        assert!(!tag_step(&mut s, "v1.0"));
        assert!(!tag_step(&mut s, "v1.0"));
        assert!(tag_step(&mut s, "v1.1"));
        fired(&mut s, "v1.1");
        assert_eq!(s.last_trigger_note, "tag v1.1");
        assert!(!tag_step(&mut s, "v1.1"));
        assert!(!tag_step(&mut s, ""));
    }

    fn git(repo: &Path, args: &[&str], date: &str) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args([
                "-c",
                "user.name=Onyx",
                "-c",
                "user.email=onyx@example.invalid",
            ])
            .args(args)
            .env("GIT_COMMITTER_DATE", date)
            .env("GIT_AUTHOR_DATE", date)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        assert!(ok, "git {args:?} failed");
    }

    #[test]
    fn newest_tag_follows_creation_date_and_pattern() {
        let repo = std::env::temp_dir().join(format!("onyx_tags_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"], "2026-01-01T10:00:00");
        git(
            &repo,
            &["commit", "-q", "--allow-empty", "-m", "one"],
            "2026-01-01T10:00:00",
        );
        git(
            &repo,
            &["tag", "-a", "v1.0", "-m", "first"],
            "2026-01-01T10:00:00",
        );
        git(
            &repo,
            &["commit", "-q", "--allow-empty", "-m", "two"],
            "2026-02-01T10:00:00",
        );
        git(
            &repo,
            &["tag", "-a", "v1.1", "-m", "second"],
            "2026-02-01T10:00:00",
        );
        git(
            &repo,
            &["tag", "-a", "nightly-5", "-m", "other"],
            "2026-03-01T10:00:00",
        );
        assert_eq!(newest_tag(&repo, "v*").unwrap().as_deref(), Some("v1.1"));
        assert_eq!(newest_tag(&repo, "").unwrap().as_deref(), Some("nightly-5"));
        assert_eq!(newest_tag(&repo, "release-*").unwrap(), None);
        let _ = std::fs::remove_dir_all(&repo);
        assert!(newest_tag(&repo, "v*").is_err());
    }

    #[test]
    fn fingerprint_changes_when_a_file_is_added() {
        let dir = std::env::temp_dir().join(format!("onyx_fp_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "one").unwrap();
        let first = folder_fingerprint(&dir).unwrap();
        std::fs::write(dir.join("b.txt"), "two").unwrap();
        let second = folder_fingerprint(&dir).unwrap();
        assert_ne!(first, second);
        assert!(second.starts_with("2:"));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(folder_fingerprint(&dir).is_err());
    }
}
