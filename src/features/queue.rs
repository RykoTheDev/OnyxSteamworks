use std::time::Duration;

use crate::model::Project;

pub const DEFAULT_MAX_RETRIES: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    Retryable,
    NoRetry,
}

const NO_RETRY: [&str; 11] = [
    "invalid password",
    "login failure",
    "failed login",
    "failed to login",
    "steam guard",
    "two-factor",
    "two factor",
    "no subscription",
    "no license",
    "invalid app",
    "access denied",
];

const NETWORK: [&str; 12] = [
    "timeout",
    "timed out",
    "connection reset",
    "connection refused",
    "connection closed",
    "could not connect",
    "no connection",
    "rate limit",
    "rate limited",
    "temporarily unavailable",
    "network error",
    "service unavailable",
];

pub fn classify_failure(output: &str, exit_code: Option<i32>) -> (RetryClass, String) {
    let l = output.to_ascii_lowercase();
    for pat in NO_RETRY {
        if l.contains(pat) {
            return (
                RetryClass::NoRetry,
                "Login or authorization problem — not retried.".to_string(),
            );
        }
    }
    if matches!(exit_code, Some(2) | Some(3)) {
        return (
            RetryClass::NoRetry,
            "Validation or login error — not retried.".to_string(),
        );
    }
    for pat in NETWORK {
        if l.contains(pat) {
            return (
                RetryClass::Retryable,
                format!("Network problem ({pat}) — will retry."),
            );
        }
    }
    (
        RetryClass::NoRetry,
        "Build failed — not a transient network error.".to_string(),
    )
}

pub fn backoff_delay(attempt: u32) -> Duration {
    match attempt {
        0 | 1 => Duration::from_secs(30),
        2 => Duration::from_secs(120),
        _ => Duration::from_secs(300),
    }
}

pub fn should_retry(class: RetryClass, attempts: u32, max: u32) -> bool {
    class == RetryClass::Retryable && attempts < max
}

pub fn effective_max(setting: u32) -> u32 {
    if setting == 0 {
        DEFAULT_MAX_RETRIES
    } else {
        setting
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JobOptions {
    pub preview: bool,
    pub backup: bool,
    pub verify: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Retrying,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn label(self) -> &'static str {
        match self {
            JobStatus::Queued => "Queued",
            JobStatus::Running => "Running",
            JobStatus::Retrying => "Retrying",
            JobStatus::Succeeded => "Succeeded",
            JobStatus::Failed => "Failed",
            JobStatus::Cancelled => "Cancelled",
        }
    }
}

pub struct Job {
    pub id: String,
    pub project: Project,
    pub branch: String,
    pub options: JobOptions,
    pub status: JobStatus,
    pub attempts: u32,
    pub max_retries: u32,
    pub retry_at: Option<std::time::Instant>,
    pub message: String,
    pub log: String,
}

impl Job {
    pub fn new(project: Project, branch: String, options: JobOptions, max_retries: u32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project,
            branch,
            options,
            status: JobStatus::Queued,
            attempts: 0,
            max_retries,
            retry_at: None,
            message: String::new(),
            log: String::new(),
        }
    }

    pub fn project_name(&self) -> String {
        if self.project.name.is_empty() {
            "Untitled".to_string()
        } else {
            self.project.name.clone()
        }
    }

    pub fn countdown_secs(&self) -> Option<u64> {
        self.retry_at.map(|t| {
            t.saturating_duration_since(std::time::Instant::now())
                .as_secs()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    fn fake_bin() -> PathBuf {
        if let Some(p) = option_env!("CARGO_BIN_EXE_fake_steamcmd") {
            return PathBuf::from(p);
        }
        let mut exe = std::env::current_exe().expect("current exe");
        exe.pop();
        if exe.file_name().is_some_and(|n| n == "deps") {
            exe.pop();
        }
        exe.push(format!("fake_steamcmd{}", std::env::consts::EXE_SUFFIX));
        exe
    }

    fn run_scenario(name: &str) -> (String, Option<i32>) {
        let out = Command::new(fake_bin())
            .arg("+login")
            .arg(name)
            .output()
            .expect("run fake");
        let mut text = String::from_utf8_lossy(&out.stdout).to_string();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        (text, out.status.code())
    }

    #[test]
    fn backoff_follows_30s_2m_5m() {
        assert_eq!(backoff_delay(1), Duration::from_secs(30));
        assert_eq!(backoff_delay(2), Duration::from_secs(120));
        assert_eq!(backoff_delay(3), Duration::from_secs(300));
        assert_eq!(backoff_delay(9), Duration::from_secs(300));
    }

    #[test]
    fn network_failures_are_retryable() {
        for scenario in ["net_reset", "net_timeout", "rate_limit"] {
            let (output, code) = run_scenario(scenario);
            let (class, _) = classify_failure(&output, code);
            assert_eq!(class, RetryClass::Retryable, "{scenario} should retry");
        }
    }

    #[test]
    fn login_and_guard_failures_are_not_retried() {
        for scenario in ["login_fail", "wrong_password"] {
            let (output, code) = run_scenario(scenario);
            let (class, _) = classify_failure(&output, code);
            assert_eq!(class, RetryClass::NoRetry, "{scenario} should not retry");
        }
        let (class, _) = classify_failure("Please enter the Steam Guard code", None);
        assert_eq!(class, RetryClass::NoRetry);
        let (vclass, _) = classify_failure("some failure", Some(2));
        assert_eq!(vclass, RetryClass::NoRetry);
    }

    #[test]
    fn should_retry_respects_attempt_cap() {
        assert!(should_retry(RetryClass::Retryable, 0, 3));
        assert!(should_retry(RetryClass::Retryable, 2, 3));
        assert!(!should_retry(RetryClass::Retryable, 3, 3));
        assert!(!should_retry(RetryClass::NoRetry, 0, 3));
    }
}
