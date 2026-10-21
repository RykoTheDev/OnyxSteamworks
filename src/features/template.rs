#![allow(dead_code)]

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::Project;

pub struct Vars {
    pub version: String,
    pub date: String,
    pub time: String,
    pub branch: String,
    pub git_commit: String,
    pub build_number: u32,
}

static GIT_CACHE: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

pub fn expand(template: &str, vars: &Vars) -> String {
    let build = vars.build_number.to_string();
    template
        .replace("{version}", &vars.version)
        .replace("{date}", &vars.date)
        .replace("{time}", &vars.time)
        .replace("{branch}", &vars.branch)
        .replace("{git_commit}", &vars.git_commit)
        .replace("{build_number}", &build)
}

pub fn resolve(project: &Project, branch: &str) -> Vars {
    let (date, time) = now_date_time();
    Vars {
        version: read_version(project),
        date,
        time,
        branch: branch.to_string(),
        git_commit: git_short(&project.content_root),
        build_number: project.build_counter.saturating_add(1),
    }
}

fn read_version(project: &Project) -> String {
    if project.version_file.trim().is_empty() {
        return String::new();
    }
    let path = project.content_root.join(project.version_file.trim());
    let Ok(text) = std::fs::read_to_string(&path) else {
        return String::new();
    };
    let trimmed = text.trim();
    if trimmed.starts_with('{')
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed)
    {
        for key in ["version", "Version", "app_version"] {
            if let Some(v) = value.get(key).and_then(|v| v.as_str()) {
                return v.to_string();
            }
        }
    }
    trimmed.lines().next().unwrap_or("").trim().to_string()
}

fn git_short(content_root: &Path) -> String {
    let key = content_root.to_string_lossy().to_string();
    if key.is_empty() {
        return String::new();
    }
    if let Ok(cache) = GIT_CACHE.lock()
        && let Some((_, v)) = cache.iter().find(|(k, _)| k == &key)
    {
        return v.clone();
    }
    let value = run_git(content_root);
    if let Ok(mut cache) = GIT_CACHE.lock() {
        cache.push((key, value.clone()));
    }
    value
}

fn run_git(content_root: &Path) -> String {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(content_root)
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => String::new(),
    }
}

fn now_date_time() -> (String, String) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (hh, mm) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32);
    (format!("{y:04}-{m:02}-{d:02}"), format!("{hh:02}{mm:02}"))
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vars {
        Vars {
            version: "1.2.3".to_string(),
            date: "2026-10-21".to_string(),
            time: "0930".to_string(),
            branch: "beta".to_string(),
            git_commit: "abc1234".to_string(),
            build_number: 42,
        }
    }

    #[test]
    fn expands_all_variables() {
        let out = expand(
            "v{version} {branch} {date}.{time} #{build_number} {git_commit}",
            &vars(),
        );
        assert_eq!(out, "v1.2.3 beta 2026-10-21.0930 #42 abc1234");
    }

    #[test]
    fn leaves_unknown_tokens_untouched() {
        assert_eq!(expand("{unknown} {version}", &vars()), "{unknown} 1.2.3");
    }

    #[test]
    fn civil_date_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(18_993), (2022, 1, 1));
        assert_eq!(civil_from_days(19_358), (2023, 1, 1));
    }
}
