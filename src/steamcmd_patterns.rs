#![allow(dead_code)]

use crate::steamcmd::{PromptKind, Stage};

pub fn detect_login(line: &str) -> Option<bool> {
    let l = line.to_ascii_lowercase();
    if (l.contains("failed") && l.contains("login"))
        || l.contains("invalid password")
        || l.contains("login failure")
        || (l.contains("access denied") && l.contains("logging in"))
        || l.contains("rate limit exceeded")
    {
        return Some(false);
    }
    if l.contains("logged in ok") || (l.contains("logging in user") && l.contains("ok")) {
        return Some(true);
    }
    None
}

pub fn login_failure_reason(line: &str) -> String {
    let l = line.to_ascii_lowercase();
    if l.contains("invalid password") {
        "Invalid password.".to_string()
    } else if l.contains("rate limit") {
        "Rate limited by Steam. Wait a few minutes and try again.".to_string()
    } else if l.contains("access denied") {
        "Steam denied the login (Access Denied). The cached credentials are invalid or expired, or the account lacks publishing access. Sign in to SteamCMD once in a terminal to refresh them, then try again.".to_string()
    } else if l.contains("expired") || (l.contains("cached") && l.contains("invalid")) {
        "The cached login expired. Run Authorize again.".to_string()
    } else if l.contains("two-factor") || l.contains("steam guard") {
        "Steam Guard code was required or incorrect.".to_string()
    } else {
        "Login failed.".to_string()
    }
}

pub fn detect_prompt(partial: &str) -> Option<PromptKind> {
    let l = partial.to_ascii_lowercase();
    if l.contains("steam guard") || l.contains("two-factor") {
        return Some(PromptKind::SteamGuard);
    }
    if l.contains("password:") {
        return Some(PromptKind::Password);
    }
    None
}

pub fn detect_stage(line: &str) -> Option<Stage> {
    let l = line.to_ascii_lowercase();
    if l.contains("connecting") {
        return Some(Stage::Connecting);
    }
    if l.contains("logging in user") {
        return Some(Stage::LoggingIn);
    }
    if l.contains("building depot") || l.contains("scanning") {
        return Some(Stage::Scanning);
    }
    if l.contains("uploading") {
        return Some(Stage::Uploading);
    }
    if l.contains("committing") {
        return Some(Stage::Committing);
    }
    None
}

pub fn detect_progress(line: &str) -> Option<f32> {
    let idx = line.find('%')?;
    let prefix = &line[..idx];
    let start = prefix
        .rfind(|c: char| !(c.is_ascii_digit() || c == '.'))
        .map(|i| i + 1)
        .unwrap_or(0);
    let value: f32 = prefix[start..].parse().ok()?;
    Some((value / 100.0).clamp(0.0, 1.0))
}

pub fn detect_success(line: &str) -> Option<String> {
    let marker = "successfully finished appid";
    let idx = line.to_ascii_lowercase().find(marker)?;
    let rest = line[idx + marker.len()..].trim_start();
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    (!digits.is_empty()).then_some(digits)
}

pub fn detect_failure(line: &str) -> Option<String> {
    let l = line.to_ascii_lowercase();
    if l.contains("error!") || l.starts_with("error ") || l.contains("failed to") {
        Some(line.trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_denied_is_a_login_failure() {
        let line =
            "Logging in user 'itsRyko11' [U:1:1251715880] to Steam Public...ERROR (Access Denied)";
        assert_eq!(detect_login(line), Some(false));
        let reason = login_failure_reason(line);
        assert!(reason.contains("Access Denied"));
    }

    #[test]
    fn plain_access_denied_without_login_context_is_not_a_login_event() {
        assert_eq!(detect_login("depot access denied for something"), None);
    }

    #[test]
    fn successful_login_still_detected() {
        assert_eq!(
            detect_login("Logging in user 'x' to Steam Public...OK"),
            Some(true)
        );
    }
}
