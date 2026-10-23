use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use crate::features::store;

const FILE: &str = "approvals.json";
pub const TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Approval {
    pub id: String,
    pub project_name: String,
    pub pipeline_name: String,
    pub message: String,
    pub requested_ms: i64,
    pub decision: Option<bool>,
}

impl Approval {
    pub fn new(project_name: &str, pipeline_name: &str, message: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_name: project_name.to_string(),
            pipeline_name: pipeline_name.to_string(),
            message: message.trim().to_string(),
            requested_ms: crate::features::schedule::now_ms(),
            decision: None,
        }
    }

    pub fn body(&self) -> String {
        let reason = if self.message.is_empty() {
            String::new()
        } else {
            format!(" ({})", self.message)
        };
        format!(
            "{} for {} is waiting for approval{reason}. Open Onyx to approve or reject it.",
            self.pipeline_name, self.project_name
        )
    }
}

pub fn stamp() -> Option<SystemTime> {
    store::stamp(FILE)
}

pub fn load() -> Vec<Approval> {
    store::load(FILE)
}

pub fn pending() -> Vec<Approval> {
    load()
        .into_iter()
        .filter(|a| a.decision.is_none())
        .collect()
}

pub fn insert(approval: Approval) {
    let mut all = load();
    all.push(approval);
    store::save(FILE, &all);
}

pub fn decide(id: &str, approve: bool) {
    let mut all = load();
    if let Some(a) = all.iter_mut().find(|a| a.id == id) {
        a.decision = Some(approve);
        store::save(FILE, &all);
    }
}

pub fn remove(id: &str) {
    let mut all = load();
    all.retain(|a| a.id != id);
    store::save(FILE, &all);
}

pub fn decision(id: &str) -> Option<bool> {
    load()
        .into_iter()
        .find(|a| a.id == id)
        .and_then(|a| a.decision)
}

pub fn wait_for_decision(
    id: &str,
    timeout: Duration,
    poll: Duration,
    check: impl Fn(&str) -> Option<bool>,
) -> Option<bool> {
    let start = Instant::now();
    loop {
        if let Some(d) = check(id) {
            return Some(d);
        }
        if start.elapsed() >= timeout {
            return None;
        }
        std::thread::sleep(poll);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn waits_until_a_decision_arrives() {
        let calls = Cell::new(0);
        let got = wait_for_decision(
            "a",
            Duration::from_secs(5),
            Duration::from_millis(1),
            |_| {
                calls.set(calls.get() + 1);
                (calls.get() >= 3).then_some(true)
            },
        );
        assert_eq!(got, Some(true));
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn times_out_without_a_decision() {
        let got = wait_for_decision(
            "a",
            Duration::from_millis(5),
            Duration::from_millis(1),
            |_| None,
        );
        assert_eq!(got, None);
    }

    #[test]
    fn body_names_the_pipeline_and_reason() {
        let a = Approval::new("Game", "Release", "Ship 1.2 to beta?");
        assert_eq!(
            a.body(),
            "Release for Game is waiting for approval (Ship 1.2 to beta?). Open Onyx to approve or reject it."
        );
    }
}
