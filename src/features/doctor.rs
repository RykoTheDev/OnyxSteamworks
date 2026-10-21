pub(crate) mod sys;

use std::path::PathBuf;
use std::process::Command;

use crate::model::{Project, StepKind};
use crate::validate::{Severity, human_size, preflight_checks};

const LOW_DISK: u64 = 5 * 1024 * 1024 * 1024;
const STEAMCMD_HELP: &str = "https://developer.valvesoftware.com/wiki/SteamCMD";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Ok => "OK",
            Status::Warn => "WARN",
            Status::Fail => "FAIL",
        }
    }

    fn max_warn(self) -> Status {
        if self == Status::Fail {
            Status::Fail
        } else {
            Status::Warn
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fix {
    OpenUrl(String),
    OpenFolder(PathBuf),
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub title: String,
    pub status: Status,
    pub message: String,
    pub fix: Option<Fix>,
}

impl CheckResult {
    fn new(title: &str, status: Status, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            status,
            message: message.into(),
            fix: None,
        }
    }

    fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }
}

pub fn mask_username(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        return "(none)".to_string();
    }
    let keep = name.chars().take(2).collect::<String>();
    let rest = name.chars().count().saturating_sub(2);
    format!("{keep}{}", "*".repeat(rest.max(1)))
}

pub fn check_steamcmd(project: Option<Project>) -> CheckResult {
    let Some(project) = project else {
        return CheckResult::new("SteamCMD", Status::Warn, "No project selected");
    };
    let path = crate::features::maintenance::effective_steamcmd(&project);
    if path.as_os_str().is_empty() || !path.is_file() {
        return CheckResult::new(
            "SteamCMD",
            Status::Fail,
            "SteamCMD was not found at the configured path",
        )
        .with_fix(Fix::OpenUrl(STEAMCMD_HELP.to_string()));
    }
    let mut cmd = crate::steamcmd::command_for(&path);
    cmd.arg("+quit");
    match sys::run_capture(cmd, 60) {
        Some((0, out)) => {
            let version = out
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("ran cleanly")
                .trim();
            CheckResult::new("SteamCMD", Status::Ok, format!("Ran OK — {version}"))
        }
        Some((code, _)) => {
            CheckResult::new("SteamCMD", Status::Warn, format!("Exited with code {code}"))
                .with_fix(Fix::OpenUrl(STEAMCMD_HELP.to_string()))
        }
        None => CheckResult::new("SteamCMD", Status::Warn, "Did not exit within 60 seconds"),
    }
}

pub fn check_internet() -> CheckResult {
    let proxy = [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ]
    .iter()
    .any(|k| std::env::var_os(k).is_some());
    let proxy_note = if proxy {
        " (proxy environment variables detected)"
    } else {
        ""
    };
    if sys::can_connect("cdn.cloudflare.steamstatic.com", 443, 8) {
        CheckResult::new(
            "Internet",
            Status::Ok,
            format!("Reached Steam CDN over HTTPS{proxy_note}"),
        )
    } else {
        CheckResult::new(
            "Internet",
            Status::Warn,
            format!("Could not reach Steam CDN on port 443{proxy_note}"),
        )
    }
}

pub fn check_disk(output: Option<PathBuf>) -> CheckResult {
    let mut targets: Vec<(String, PathBuf)> = vec![("temp".to_string(), std::env::temp_dir())];
    if let Some(out) = output.filter(|p| !p.as_os_str().is_empty()) {
        targets.insert(0, ("output".to_string(), out));
    }
    let mut status = Status::Ok;
    let mut notes = Vec::new();
    for (label, path) in &targets {
        match sys::free_space(path) {
            Some(bytes) => {
                notes.push(format!("{label}: {} free", human_size(bytes)));
                if bytes < LOW_DISK {
                    status = Status::Warn;
                }
            }
            None => {
                notes.push(format!("{label}: unknown"));
                status = status.max_warn();
            }
        }
    }
    let result = CheckResult::new("Disk space", status, notes.join(", "));
    match targets.first() {
        Some((_, path)) if status != Status::Ok => result.with_fix(Fix::OpenFolder(path.clone())),
        _ => result,
    }
}

pub fn check_write_access(output: Option<PathBuf>) -> CheckResult {
    let mut targets: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(data) = crate::config::data_dir() {
        targets.push(("app data".to_string(), data));
    }
    if let Some(out) = output.filter(|p| !p.as_os_str().is_empty()) {
        targets.push(("output".to_string(), out));
    }
    let mut failed = Vec::new();
    for (label, path) in &targets {
        if !sys::writable(path) {
            failed.push(label.clone());
        }
    }
    if failed.is_empty() {
        CheckResult::new(
            "Write access",
            Status::Ok,
            "Output and app data folders are writable",
        )
    } else {
        let folders = failed.join(", ");
        let mut result = CheckResult::new(
            "Write access",
            Status::Fail,
            format!("Not writable: {folders}"),
        );
        if let Some((_, path)) = targets.first() {
            result = result.with_fix(Fix::OpenFolder(path.clone()));
        }
        result
    }
}

pub fn check_cached_login(project: Option<Project>) -> CheckResult {
    let Some(project) = project else {
        return CheckResult::new("Cached login", Status::Warn, "No project selected");
    };
    let user = project.steam_username.trim();
    if user.is_empty() {
        return CheckResult::new("Cached login", Status::Warn, "No Steam username configured");
    }
    let exe = crate::features::maintenance::effective_steamcmd(&project);
    if crate::features::accounts::is_authorized(&exe, user) {
        CheckResult::new(
            "Cached login",
            Status::Ok,
            format!("Cached SteamCMD login present for {}", mask_username(user)),
        )
    } else {
        CheckResult::new(
            "Cached login",
            Status::Warn,
            format!(
                "No cached login for {}; run Authorize once",
                mask_username(user)
            ),
        )
    }
}

pub fn check_project(project: Option<Project>) -> CheckResult {
    let Some(project) = project else {
        return CheckResult::new("Project config", Status::Warn, "No project selected");
    };
    let issues = preflight_checks(&project);
    if let Some(err) = issues.iter().find(|i| i.severity == Severity::Error) {
        CheckResult::new("Project config", Status::Fail, err.message.clone())
    } else if let Some(warn) = issues.iter().find(|i| i.severity == Severity::Warning) {
        CheckResult::new("Project config", Status::Warn, warn.message.clone())
    } else {
        CheckResult::new(
            "Project config",
            Status::Ok,
            "All pre-flight validators passed",
        )
    }
}

pub fn check_tools(project: Option<Project>) -> CheckResult {
    let mut status = Status::Ok;
    let mut notes = Vec::new();
    let mut git = Command::new("git");
    git.arg("--version");
    match sys::run_capture(git, 10) {
        Some((0, out)) => notes.push(out.lines().next().unwrap_or("git").trim().to_string()),
        _ => {
            notes.push("git not found".to_string());
            status = Status::Warn;
        }
    }
    if let Some(project) = project {
        for step in project
            .pipeline
            .iter()
            .filter(|s| s.kind == StepKind::RunCommand)
        {
            let exe = step.exe.trim();
            if exe.is_empty() || !(exe.contains('/') || exe.contains('\\')) {
                continue;
            }
            if !std::path::Path::new(exe).is_file() {
                notes.push(format!("missing tool: {exe}"));
                status = Status::Warn;
            }
        }
    }
    CheckResult::new("Build tools", status, notes.join("; "))
}

pub fn overall(results: &[CheckResult]) -> Status {
    if results.iter().any(|r| r.status == Status::Fail) {
        Status::Fail
    } else if results.iter().any(|r| r.status == Status::Warn) {
        Status::Warn
    } else {
        Status::Ok
    }
}

pub fn last_log_lines(limit: usize) -> String {
    let Ok(dir) = crate::config::log_dir() else {
        return String::new();
    };
    let latest = std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("log"))
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
    let Some(entry) = latest else {
        return String::new();
    };
    let Ok(text) = std::fs::read_to_string(entry.path()) else {
        return String::new();
    };
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(limit)..].join("\n")
}

pub fn diagnostics_report(username: &str, results: &[CheckResult]) -> String {
    let mut out = String::new();
    out.push_str("Onyx diagnostics\n");
    out.push_str(&format!(
        "OS: {} {}\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    out.push_str(&format!("App: onyx {}\n", env!("CARGO_PKG_VERSION")));
    out.push_str(&format!("Steam user: {}\n", mask_username(username)));
    out.push_str("\n--- Checks ---\n");
    for r in results {
        out.push_str(&format!(
            "[{}] {} — {}\n",
            r.status.label(),
            r.title,
            r.message
        ));
    }
    out.push_str("\n--- Last log (up to 100 lines) ---\n");
    let log = last_log_lines(100);
    let masked_log = if username.trim().is_empty() {
        log
    } else {
        log.replace(username.trim(), &mask_username(username))
    };
    out.push_str(&masked_log);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_username_without_revealing_it() {
        assert_eq!(mask_username("tester"), "te****");
        assert_eq!(mask_username("a"), "a*");
        assert_eq!(mask_username(""), "(none)");
    }

    #[test]
    fn overall_picks_worst_status() {
        let ok = CheckResult::new("a", Status::Ok, "");
        let warn = CheckResult::new("b", Status::Warn, "");
        let fail = CheckResult::new("c", Status::Fail, "");
        assert_eq!(overall(&[ok.clone(), warn.clone()]), Status::Warn);
        assert_eq!(overall(&[ok.clone(), warn, fail]), Status::Fail);
        assert_eq!(overall(&[ok.clone(), ok]), Status::Ok);
    }

    #[test]
    fn missing_steamcmd_fails_with_a_fix() {
        let project = Project {
            steamcmd_path: "/nope/steamcmd".into(),
            ..Default::default()
        };
        let result = check_steamcmd(Some(project));
        assert_eq!(result.status, Status::Fail);
        assert!(matches!(result.fix, Some(Fix::OpenUrl(_))));
    }

    #[test]
    fn diagnostics_masks_user_and_lists_checks() {
        let results = vec![CheckResult::new(
            "Disk space",
            Status::Ok,
            "temp: 20.0 GB free",
        )];
        let report = diagnostics_report("tester", &results);
        assert!(report.contains("Steam user: te****"));
        assert!(report.contains("[OK] Disk space"));
        assert!(!report.contains("tester"));
    }
}
