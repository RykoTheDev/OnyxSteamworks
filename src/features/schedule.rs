use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const DAY_MS: i64 = 86_400_000;
const WEEK_MS: i64 = 7 * DAY_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Recurrence {
    #[default]
    Once,
    Daily,
    Weekly,
}

impl Recurrence {
    pub fn label(self) -> &'static str {
        match self {
            Recurrence::Once => "Once",
            Recurrence::Daily => "Daily",
            Recurrence::Weekly => "Weekly",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Schedule {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub branch: String,
    pub preview: bool,
    pub recurrence: Recurrence,
    pub next_ms: i64,
    pub enabled: bool,
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub struct ScheduleForm {
    pub project_sel: usize,
    pub recurrence: Recurrence,
    pub hour: u32,
    pub minute: u32,
    pub weekday: u32,
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub inited: bool,
}

impl Default for ScheduleForm {
    fn default() -> Self {
        Self { project_sel: 0, recurrence: Recurrence::Once, hour: 9, minute: 0, weekday: 1, year: 2026, month: 1, day: 1, inited: false }
    }
}

impl ScheduleForm {
    pub fn seed(&mut self, now: i64) {
        if self.inited {
            return;
        }
        let days = now.div_euclid(86_400_000);
        let (y, m, d) = civil_from_days(days);
        self.year = y;
        self.month = m;
        self.day = d;
        self.weekday = weekday(days);
        self.inited = true;
    }

    pub fn next_ms(&self) -> i64 {
        match self.recurrence {
            Recurrence::Once => epoch_ms(self.year, self.month, self.day, self.hour, self.minute),
            Recurrence::Daily => next_daily(now_ms(), self.hour, self.minute),
            Recurrence::Weekly => next_weekly(now_ms(), self.weekday, self.hour, self.minute),
        }
    }
}

pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
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

pub fn weekday(days: i64) -> u32 {
    (((days % 7) + 11) % 7) as u32
}

pub fn epoch_ms(y: i64, m: u32, d: u32, h: u32, min: u32) -> i64 {
    days_from_civil(y, m, d) * DAY_MS + (h as i64 * 3600 + min as i64 * 60) * 1000
}

pub fn next_daily(now: i64, h: u32, min: u32) -> i64 {
    let days = now.div_euclid(DAY_MS);
    let (y, m, d) = civil_from_days(days);
    let today = epoch_ms(y, m, d, h, min);
    if today > now { today } else { today + DAY_MS }
}

pub fn next_weekly(now: i64, target_dow: u32, h: u32, min: u32) -> i64 {
    let mut candidate = next_daily(now, h, min);
    for _ in 0..8 {
        if weekday(candidate.div_euclid(DAY_MS)) == target_dow && candidate > now {
            return candidate;
        }
        candidate += DAY_MS;
    }
    candidate
}

pub fn advance(next_ms: i64, recurrence: Recurrence) -> Option<i64> {
    match recurrence {
        Recurrence::Once => None,
        Recurrence::Daily => Some(next_ms + DAY_MS),
        Recurrence::Weekly => Some(next_ms + WEEK_MS),
    }
}

pub fn fmt_dt(ms: i64) -> String {
    let days = ms.div_euclid(DAY_MS);
    let (y, m, d) = civil_from_days(days);
    let rem = ms.rem_euclid(DAY_MS) / 1000;
    let (h, min) = (rem / 3600, (rem % 3600) / 60);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02} UTC")
}

pub struct TaskTiming {
    pub rec: Recurrence,
    pub hour: u32,
    pub minute: u32,
    pub date: Option<(i64, u32, u32)>,
    pub dow: u32,
}

pub fn schtasks_create_args(task: &str, exe: &str, build_args: &[String], t: &TaskTiming) -> Vec<String> {
    let tr = {
        let mut parts = vec![quote(exe)];
        parts.extend(build_args.iter().map(|a| quote(a)));
        parts.join(" ")
    };
    let mut args = vec!["/Create".into(), "/TN".into(), task.to_string(), "/TR".into(), tr];
    match t.rec {
        Recurrence::Once => {
            args.push("/SC".into());
            args.push("ONCE".into());
            if let Some((y, m, d)) = t.date {
                args.push("/SD".into());
                args.push(format!("{m:02}/{d:02}/{y:04}"));
            }
        }
        Recurrence::Daily => {
            args.push("/SC".into());
            args.push("DAILY".into());
        }
        Recurrence::Weekly => {
            args.push("/SC".into());
            args.push("WEEKLY".into());
            args.push("/D".into());
            args.push(dow_token(t.dow).into());
        }
    }
    args.push("/ST".into());
    args.push(format!("{:02}:{:02}", t.hour, t.minute));
    args.push("/F".into());
    args
}

pub fn schtasks_delete_args(task: &str) -> Vec<String> {
    vec!["/Delete".into(), "/TN".into(), task.to_string(), "/F".into()]
}

fn dow_token(dow: u32) -> &'static str {
    ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"][(dow % 7) as usize]
}

fn quote(value: &str) -> String {
    if value.contains(' ') { format!("\"{value}\"") } else { value.to_string() }
}

fn schedules_path() -> Option<PathBuf> {
    crate::config::data_dir().ok().map(|d| d.join("schedules.json"))
}

pub fn load() -> Vec<Schedule> {
    schedules_path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save(schedules: &[Schedule]) {
    if let Some(path) = schedules_path()
        && let Ok(bytes) = serde_json::to_vec_pretty(schedules)
    {
        let _ = std::fs::write(path, bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trips() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        let days = days_from_civil(2026, 10, 21);
        assert_eq!(civil_from_days(days), (2026, 10, 21));
    }

    #[test]
    fn weekday_known_date() {
        assert_eq!(weekday(days_from_civil(1970, 1, 1)), 4);
        assert_eq!(weekday(days_from_civil(2026, 10, 21)), 3);
    }

    #[test]
    fn daily_rolls_to_tomorrow_when_past() {
        let base = epoch_ms(2026, 10, 21, 12, 0);
        let before = next_daily(base - 3600_000, 12, 0);
        assert_eq!(before, base);
        let after = next_daily(base + 3600_000, 12, 0);
        assert_eq!(after, base + DAY_MS);
    }

    #[test]
    fn advance_disables_once_and_steps_recurring() {
        assert_eq!(advance(100, Recurrence::Once), None);
        assert_eq!(advance(0, Recurrence::Daily), Some(DAY_MS));
        assert_eq!(advance(0, Recurrence::Weekly), Some(WEEK_MS));
    }

    #[test]
    fn schtasks_args_build_create_and_delete() {
        let timing = TaskTiming { rec: Recurrence::Weekly, hour: 9, minute: 30, date: None, dow: 1 };
        let args = schtasks_create_args("Onyx Game", "C:/app/onyx.exe", &["build".into(), "--project".into(), "My Game".into()], &timing);
        assert!(args.contains(&"/Create".to_string()));
        assert!(args.contains(&"WEEKLY".to_string()));
        assert!(args.contains(&"MON".to_string()));
        assert!(args.contains(&"09:30".to_string()));
        assert!(args.iter().any(|a| a.contains("\"My Game\"")));
        assert_eq!(schtasks_delete_args("Onyx Game"), vec!["/Delete", "/TN", "Onyx Game", "/F"]);
    }
}
