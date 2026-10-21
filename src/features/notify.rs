use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use eframe::egui;

use crate::model::BuildStatus;

pub const WEBHOOK_KEY: &str = "discord:webhook";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failed,
    Cancelled,
}

impl Outcome {
    pub fn from_status(status: BuildStatus) -> Option<Self> {
        match status {
            BuildStatus::Success => Some(Outcome::Success),
            BuildStatus::Failed => Some(Outcome::Failed),
            BuildStatus::Cancelled => Some(Outcome::Cancelled),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Outcome::Success => "succeeded",
            Outcome::Failed => "failed",
            Outcome::Cancelled => "was cancelled",
        }
    }

    fn mark(self) -> &'static str {
        match self {
            Outcome::Success => "\u{2705}",
            Outcome::Failed => "\u{274c}",
            Outcome::Cancelled => "\u{26a0}",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildInfo {
    pub outcome: Outcome,
    pub project: String,
    pub number: u32,
    pub duration_ms: u64,
    pub branch: String,
    pub notes: Option<String>,
}

fn fmt_duration(ms: u64) -> String {
    let sec = ms / 1000;
    format!("{:02}:{:02}", sec / 60, sec % 60)
}

impl BuildInfo {
    pub fn summary(&self) -> String {
        format!(
            "{} build #{} {}",
            self.project,
            self.number,
            self.outcome.word()
        )
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn body(&self) -> String {
        let branch = if self.branch.is_empty() {
            "default".to_string()
        } else {
            self.branch.clone()
        };
        format!("Branch {branch} · {}", fmt_duration(self.duration_ms))
    }

    pub fn discord_content(&self) -> String {
        let branch = if self.branch.is_empty() {
            "default"
        } else {
            self.branch.as_str()
        };
        let mut out = format!(
            "{} **{} — build #{} {}**\nBranch: {branch} · Duration: {}",
            self.outcome.mark(),
            self.project,
            self.number,
            self.outcome.word(),
            fmt_duration(self.duration_ms),
        );
        if let Some(notes) = self
            .notes
            .as_ref()
            .map(|n| n.trim())
            .filter(|n| !n.is_empty())
        {
            out.push_str("\n\n");
            out.push_str(notes);
        }
        if out.chars().count() > 1900 {
            out = out.chars().take(1900).collect::<String>();
            out.push_str("\n…");
        }
        out
    }
}

pub fn should_notify(outcome: Outcome, on_success: bool, on_failure: bool) -> bool {
    match outcome {
        Outcome::Success => on_success,
        Outcome::Failed | Outcome::Cancelled => on_failure,
    }
}

#[cfg(windows)]
pub fn os_toast_message(summary: &str, body: &str) -> anyhow::Result<()> {
    notify_rust::Notification::new()
        .summary(summary)
        .body(body)
        .show()?;
    Ok(())
}

#[cfg(not(windows))]
pub fn os_toast_message(_summary: &str, _body: &str) -> anyhow::Result<()> {
    Ok(())
}

pub fn os_toast(info: &BuildInfo) -> anyhow::Result<()> {
    os_toast_message(&info.summary(), &info.body())
}

pub fn post_discord_text(webhook: &str, content: &str) -> Result<(), String> {
    let webhook = webhook.trim();
    if webhook.is_empty() {
        return Err("No Discord webhook is set".to_string());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build();
    let body = serde_json::json!({ "content": content });
    agent
        .post(webhook)
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn post_discord(webhook: &str, info: &BuildInfo) -> Result<(), String> {
    post_discord_text(webhook, &info.discord_content())
}

pub struct NotifyState {
    tx: Sender<String>,
    rx: Receiver<String>,
}

impl NotifyState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx }
    }

    pub fn fire(&self, info: BuildInfo, ctx: &egui::Context) {
        let cfg = crate::config::load_settings().unwrap_or_default();
        let want = should_notify(info.outcome, cfg.notify_success, cfg.notify_failure);
        if !want {
            return;
        }
        let toast = cfg.notify_toast;
        let discord = cfg.discord_enabled;
        if !toast && !discord {
            return;
        }
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if toast && let Err(e) = os_toast(&info) {
                let _ = tx.send(format!("Toast failed: {e}"));
            }
            if discord {
                let webhook = crate::secrets::get_password(WEBHOOK_KEY)
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                if let Err(e) = post_discord(&webhook, &info) {
                    let _ = tx.send(format!("Discord failed: {e}"));
                }
            }
            ctx.request_repaint();
        });
    }

    pub fn post_text(&self, webhook: String, content: String, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if let Err(e) = post_discord_text(&webhook, &content) {
                let _ = tx.send(format!("Discord failed: {e}"));
            }
            ctx.request_repaint();
        });
    }

    pub fn drain(&self) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            out.push(msg);
        }
        out
    }
}

impl Default for NotifyState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn test_message() -> BuildInfo {
    BuildInfo {
        outcome: Outcome::Success,
        project: "SteamPipe GUI".to_string(),
        number: 0,
        duration_ms: 0,
        branch: "test".to_string(),
        notes: Some("This is a test message from SteamPipe GUI.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> BuildInfo {
        BuildInfo {
            outcome: Outcome::Success,
            project: "Nebula Drift".into(),
            number: 12,
            duration_ms: 151_000,
            branch: "beta".into(),
            notes: Some("## Fixes\n- a crash".into()),
        }
    }

    #[test]
    fn summary_and_body_have_number_and_duration() {
        let i = info();
        assert_eq!(i.summary(), "Nebula Drift build #12 succeeded");
        assert_eq!(i.body(), "Branch beta · 02:31");
    }

    #[test]
    fn discord_content_includes_notes_and_fields() {
        let c = info().discord_content();
        assert!(c.contains("build #12 succeeded"));
        assert!(c.contains("Branch: beta"));
        assert!(c.contains("Duration: 02:31"));
        assert!(c.contains("a crash"));
    }

    #[test]
    fn filter_respects_toggles() {
        assert!(should_notify(Outcome::Success, true, false));
        assert!(!should_notify(Outcome::Success, false, true));
        assert!(should_notify(Outcome::Failed, false, true));
        assert!(should_notify(Outcome::Cancelled, false, true));
    }

    #[test]
    fn discord_content_is_truncated() {
        let mut i = info();
        i.notes = Some("x".repeat(5000));
        let c = i.discord_content();
        assert!(c.chars().count() <= 1902);
    }
}
