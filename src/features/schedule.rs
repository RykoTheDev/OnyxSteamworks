use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::model::RunOutcome;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
pub const GRACE_MS: i64 = 5 * MINUTE_MS;
pub const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAY_XML: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Recurrence {
    #[default]
    Once,
    Daily,
    Weekly,
}

impl Recurrence {
    pub const ALL: [Recurrence; 3] = [Recurrence::Once, Recurrence::Daily, Recurrence::Weekly];

    pub fn label(self) -> &'static str {
        match self {
            Recurrence::Once => "Once",
            Recurrence::Daily => "Daily",
            Recurrence::Weekly => "Weekly",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TriggerKind {
    #[default]
    Time,
    FolderChange,
    GitTag,
}

impl TriggerKind {
    pub const ALL: [TriggerKind; 3] = [
        TriggerKind::Time,
        TriggerKind::FolderChange,
        TriggerKind::GitTag,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TriggerKind::Time => "On a schedule",
            TriggerKind::FolderChange => "When files change",
            TriggerKind::GitTag => "When a git tag appears",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MissedPolicy {
    #[default]
    CatchUp,
    Skip,
}

impl MissedPolicy {
    pub const ALL: [MissedPolicy; 2] = [MissedPolicy::CatchUp, MissedPolicy::Skip];

    pub fn label(self) -> &'static str {
        match self {
            MissedPolicy::CatchUp => "Catch up once",
            MissedPolicy::Skip => "Skip missed runs",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Schedule {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub trigger: TriggerKind,
    pub watch_path: String,
    pub quiet_minutes: u32,
    pub tag_pattern: String,
    pub baseline: String,
    pub pending: String,
    pub pending_since_ms: i64,
    pub last_trigger_note: String,
    pub recurrence: Recurrence,
    pub hour: u32,
    pub minute: u32,
    pub weekday: u32,
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub missed: MissedPolicy,
    pub enabled: bool,
    pub task_installed: bool,
    pub next_ms: i64,
    pub last_started_ms: i64,
    pub last_finished_ms: i64,
    pub last_outcome: Option<RunOutcome>,
}

impl Schedule {
    pub fn is_legacy(&self) -> bool {
        self.pipeline_id.is_empty()
    }

    pub fn is_event(&self) -> bool {
        self.trigger != TriggerKind::Time
    }

    pub fn describe(&self) -> String {
        let place = if self.watch_path.trim().is_empty() {
            String::new()
        } else {
            format!(" in {}", self.watch_path.trim())
        };
        match self.trigger {
            TriggerKind::FolderChange => {
                return format!(
                    "When files change{place}, after {} quiet min",
                    self.quiet_minutes.max(1)
                );
            }
            TriggerKind::GitTag => {
                let pattern = if self.tag_pattern.trim().is_empty() {
                    "*"
                } else {
                    self.tag_pattern.trim()
                };
                return format!("When a new tag matching {pattern} appears{place}");
            }
            TriggerKind::Time => {}
        }
        let at = format!("{:02}:{:02}", self.hour, self.minute);
        match self.recurrence {
            Recurrence::Once => format!(
                "Once on {} {} {} at {at}",
                self.day,
                MONTHS[(self.month.clamp(1, 12) - 1) as usize],
                self.year
            ),
            Recurrence::Daily => format!("Daily at {at}"),
            Recurrence::Weekly => {
                format!("Every {} at {at}", WEEKDAYS[(self.weekday % 7) as usize])
            }
        }
    }

    pub fn task_name(&self) -> String {
        let clean = |s: &str| -> String {
            s.chars()
                .map(|c| if "\\/:*?\"<>|".contains(c) { '-' } else { c })
                .collect::<String>()
                .trim()
                .to_string()
        };
        let short: String = self.id.chars().take(8).collect();
        format!(
            "Onyx\\{} - {} ({short})",
            clean(&self.project_name),
            clean(&self.pipeline_name)
        )
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(windows)]
pub fn local_offset_ms(utc_ms: i64) -> i64 {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::Time::SystemTimeToTzSpecificLocalTime;
    let whole = utc_ms - utc_ms.rem_euclid(1000);
    let (y, m, d) = civil_from_days(whole.div_euclid(DAY_MS));
    let rem = whole.rem_euclid(DAY_MS);
    let utc = SYSTEMTIME {
        wYear: y as u16,
        wMonth: m as u16,
        wDayOfWeek: 0,
        wDay: d as u16,
        wHour: (rem / 3_600_000) as u16,
        wMinute: ((rem / MINUTE_MS) % 60) as u16,
        wSecond: ((rem / 1000) % 60) as u16,
        wMilliseconds: 0,
    };
    let mut local = SYSTEMTIME {
        wYear: 0,
        wMonth: 0,
        wDayOfWeek: 0,
        wDay: 0,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    };
    let ok = unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) };
    if ok == 0 {
        return 0;
    }
    let local_ms = epoch_ms(
        local.wYear as i64,
        local.wMonth as u32,
        local.wDay as u32,
        local.wHour as u32,
        local.wMinute as u32,
    ) + local.wSecond as i64 * 1000;
    local_ms - whole
}

#[cfg(unix)]
pub fn local_offset_ms(utc_ms: i64) -> i64 {
    let t = utc_ms.div_euclid(1000) as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let res = unsafe { libc::localtime_r(&t, &mut tm) };
    if res.is_null() {
        0
    } else {
        tm.tm_gmtoff as i64 * 1000
    }
}

#[cfg(not(any(windows, unix)))]
pub fn local_offset_ms(_utc_ms: i64) -> i64 {
    0
}

fn to_utc(local: i64, offset: &dyn Fn(i64) -> i64) -> i64 {
    local - offset(local - offset(local))
}

pub fn next_after(s: &Schedule, after: i64, offset: &dyn Fn(i64) -> i64) -> Option<i64> {
    if s.is_event() {
        return None;
    }
    let now_local = after + offset(after);
    let next = match s.recurrence {
        Recurrence::Once => to_utc(epoch_ms(s.year, s.month, s.day, s.hour, s.minute), offset),
        Recurrence::Daily => to_utc(next_daily(now_local, s.hour, s.minute), offset),
        Recurrence::Weekly => to_utc(next_weekly(now_local, s.weekday, s.hour, s.minute), offset),
    };
    (next > after).then_some(next)
}

pub fn next_run(s: &Schedule, after: i64) -> Option<i64> {
    next_after(s, after, &local_offset_ms)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    No,
    Run { missed: bool },
    Skip,
}

pub fn due(s: &Schedule, now: i64) -> Due {
    if !s.enabled
        || s.is_legacy()
        || s.is_event()
        || s.task_installed
        || s.next_ms == 0
        || s.next_ms > now
    {
        return Due::No;
    }
    if now - s.next_ms <= GRACE_MS {
        return Due::Run { missed: false };
    }
    match s.missed {
        MissedPolicy::CatchUp => Due::Run { missed: true },
        MissedPolicy::Skip => Due::Skip,
    }
}

pub fn advance(s: &mut Schedule, now: i64) {
    if s.is_event() {
        s.next_ms = 0;
        return;
    }
    match next_run(s, now) {
        Some(next) => s.next_ms = next,
        None => {
            s.next_ms = 0;
            if s.recurrence == Recurrence::Once {
                s.enabled = false;
            }
        }
    }
}

pub fn fmt_local(ms: i64) -> String {
    if ms <= 0 {
        return "not scheduled".to_string();
    }
    let local = ms + local_offset_ms(ms);
    let days = local.div_euclid(DAY_MS);
    let (_, m, d) = civil_from_days(days);
    let rem = local.rem_euclid(DAY_MS) / MINUTE_MS;
    format!(
        "{} {d} {}, {:02}:{:02}",
        WEEKDAYS[weekday(days) as usize],
        MONTHS[(m - 1) as usize],
        rem / 60,
        rem % 60
    )
}

pub fn local_today() -> (i64, u32, u32, u32) {
    let now = now_ms();
    let days = (now + local_offset_ms(now)).div_euclid(DAY_MS);
    let (y, m, d) = civil_from_days(days);
    (y, m, d, weekday(days))
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

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn task_xml(s: &Schedule, exe: &str, start_date: (i64, u32, u32)) -> String {
    let (y, m, d) = match s.recurrence {
        Recurrence::Once => (s.year, s.month, s.day),
        _ => start_date,
    };
    let start = format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:00", s.hour, s.minute);
    let trigger = match (s.trigger, s.recurrence) {
        (TriggerKind::FolderChange | TriggerKind::GitTag, _) => format!(
            "<TimeTrigger><StartBoundary>{y:04}-{m:02}-{d:02}T00:00:00</StartBoundary><Enabled>true</Enabled><Repetition><Interval>PT15M</Interval><StopAtDurationEnd>false</StopAtDurationEnd></Repetition></TimeTrigger>"
        ),
        (_, Recurrence::Once) => format!(
            "<TimeTrigger><StartBoundary>{start}</StartBoundary><Enabled>true</Enabled></TimeTrigger>"
        ),
        (_, Recurrence::Daily) => format!(
            "<CalendarTrigger><StartBoundary>{start}</StartBoundary><Enabled>true</Enabled><ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay></CalendarTrigger>"
        ),
        (_, Recurrence::Weekly) => format!(
            "<CalendarTrigger><StartBoundary>{start}</StartBoundary><Enabled>true</Enabled><ScheduleByWeek><DaysOfWeek><{day} /></DaysOfWeek><WeeksInterval>1</WeeksInterval></ScheduleByWeek></CalendarTrigger>",
            day = WEEKDAY_XML[(s.weekday % 7) as usize]
        ),
    };
    let dir = Path::new(exe)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>{desc}</Description>
  </RegistrationInfo>
  <Triggers>
    {trigger}
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <StartWhenAvailable>{catch_up}</StartWhenAvailable>
    <ExecutionTimeLimit>PT12H</ExecutionTimeLimit>
    <Enabled>{enabled}</Enabled>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{command}</Command>
      <Arguments>pipeline run --schedule {id}{extra}</Arguments>
      <WorkingDirectory>{dir}</WorkingDirectory>
    </Exec>
  </Actions>
</Task>
"#,
        desc = xml_escape(&format!(
            "Onyx runs the {} pipeline for {}. {}.",
            s.pipeline_name,
            s.project_name,
            s.describe()
        )),
        catch_up = s.missed == MissedPolicy::CatchUp || s.is_event(),
        extra = if s.is_event() { " --if-triggered" } else { "" },
        enabled = s.enabled,
        command = xml_escape(exe),
        id = xml_escape(&s.id),
        dir = xml_escape(&dir),
    )
}

fn run_schtasks(args: &[String]) -> Result<(), String> {
    let mut cmd = std::process::Command::new("schtasks");
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("Could not run schtasks: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if msg.is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            msg
        })
    }
}

pub fn install_task(s: &Schedule) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Task Scheduler is only available on Windows.".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let (y, m, d, _) = local_today();
    let xml = task_xml(s, &exe.to_string_lossy(), (y, m, d));
    let path = std::env::temp_dir().join(format!("onyx-task-{}.xml", s.id));
    let mut bytes = vec![0xFF, 0xFE];
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    let result = run_schtasks(&[
        "/Create".into(),
        "/TN".into(),
        s.task_name(),
        "/XML".into(),
        path.to_string_lossy().to_string(),
        "/F".into(),
    ]);
    let _ = std::fs::remove_file(&path);
    result
}

pub fn remove_task(s: &Schedule) -> Result<(), String> {
    run_schtasks(&["/Delete".into(), "/TN".into(), s.task_name(), "/F".into()])
}

pub fn set_task_enabled(s: &Schedule, enabled: bool) -> Result<(), String> {
    run_schtasks(&[
        "/Change".into(),
        "/TN".into(),
        s.task_name(),
        if enabled { "/ENABLE" } else { "/DISABLE" }.into(),
    ])
}

const FILE: &str = "schedules.json";

pub fn file_stamp() -> Option<SystemTime> {
    crate::features::store::stamp(FILE)
}

pub fn load() -> Vec<Schedule> {
    crate::features::store::load(FILE)
}

pub fn save(schedules: &[Schedule]) {
    crate::features::store::save(FILE, schedules);
}

pub fn update(id: &str, change: impl FnOnce(&mut Schedule)) -> Vec<Schedule> {
    let mut all = load();
    if let Some(s) = all.iter_mut().find(|s| s.id == id) {
        change(s);
        save(&all);
    }
    all
}

pub fn insert(schedule: Schedule) -> Vec<Schedule> {
    let mut all = load();
    all.push(schedule);
    save(&all);
    all
}

pub fn remove(id: &str) -> Vec<Schedule> {
    let mut all = load();
    all.retain(|s| s.id != id);
    save(&all);
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    const PKT: i64 = 5 * 3_600_000;

    fn fixed(_: i64) -> i64 {
        PKT
    }

    fn schedule(recurrence: Recurrence) -> Schedule {
        Schedule {
            id: "abcdef1234".into(),
            project_name: "My: Game".into(),
            pipeline_id: "pipe".into(),
            pipeline_name: "Nightly".into(),
            recurrence,
            hour: 9,
            minute: 30,
            weekday: 1,
            year: 2026,
            month: 10,
            day: 26,
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn civil_round_trips() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        let days = days_from_civil(2026, 10, 21);
        assert_eq!(civil_from_days(days), (2026, 10, 21));
        assert_eq!(weekday(days_from_civil(2026, 10, 21)), 3);
    }

    #[test]
    fn daily_uses_local_wall_clock_time() {
        let s = schedule(Recurrence::Daily);
        let after = epoch_ms(2026, 10, 23, 3, 0);
        assert_eq!(
            next_after(&s, after, &fixed),
            Some(epoch_ms(2026, 10, 23, 4, 30))
        );
        let later = epoch_ms(2026, 10, 23, 5, 0);
        assert_eq!(
            next_after(&s, later, &fixed),
            Some(epoch_ms(2026, 10, 24, 4, 30))
        );
    }

    #[test]
    fn weekly_lands_on_the_local_weekday() {
        let s = schedule(Recurrence::Weekly);
        let friday = epoch_ms(2026, 10, 23, 12, 0);
        let next = next_after(&s, friday, &fixed).expect("next");
        assert_eq!(next, epoch_ms(2026, 10, 26, 4, 30));
        assert_eq!(weekday((next + PKT).div_euclid(DAY_MS)), 1);
    }

    #[test]
    fn once_in_the_past_has_no_next_run() {
        let s = schedule(Recurrence::Once);
        assert_eq!(
            next_after(&s, epoch_ms(2026, 10, 20, 0, 0), &fixed),
            Some(epoch_ms(2026, 10, 26, 4, 30))
        );
        assert_eq!(next_after(&s, epoch_ms(2026, 11, 1, 0, 0), &fixed), None);
    }

    #[test]
    fn due_applies_grace_and_missed_policy() {
        let mut s = schedule(Recurrence::Daily);
        s.next_ms = 1_000_000;
        assert_eq!(due(&s, 999_999), Due::No);
        assert_eq!(due(&s, 1_000_000 + GRACE_MS), Due::Run { missed: false });
        let late = 1_000_000 + GRACE_MS + 1;
        assert_eq!(due(&s, late), Due::Run { missed: true });
        s.missed = MissedPolicy::Skip;
        assert_eq!(due(&s, late), Due::Skip);
        s.task_installed = true;
        assert_eq!(due(&s, late), Due::No);
        s.task_installed = false;
        s.pipeline_id.clear();
        assert_eq!(due(&s, late), Due::No);
    }

    #[test]
    fn task_xml_is_locale_free_and_escaped() {
        let mut s = schedule(Recurrence::Weekly);
        s.missed = MissedPolicy::Skip;
        let xml = task_xml(&s, r"C:\Apps\R&D\onyx.exe", (2026, 10, 23));
        assert!(xml.contains("<StartBoundary>2026-10-23T09:30:00</StartBoundary>"));
        assert!(xml.contains("<Monday />"));
        assert!(xml.contains("<StartWhenAvailable>false</StartWhenAvailable>"));
        assert!(xml.contains(r"<Command>C:\Apps\R&amp;D\onyx.exe</Command>"));
        assert!(xml.contains("<Arguments>pipeline run --schedule abcdef1234</Arguments>"));
        let once = task_xml(&schedule(Recurrence::Once), "onyx.exe", (2000, 1, 1));
        assert!(once.contains("<TimeTrigger><StartBoundary>2026-10-26T09:30:00"));
    }

    #[test]
    fn event_schedules_poll_every_15_minutes_and_never_come_due() {
        let mut s = schedule(Recurrence::Daily);
        s.trigger = TriggerKind::GitTag;
        s.tag_pattern = "v*".into();
        let xml = task_xml(&s, "onyx.exe", (2026, 10, 23));
        assert!(xml.contains("<Interval>PT15M</Interval>"));
        assert!(xml.contains("<StartBoundary>2026-10-23T00:00:00</StartBoundary>"));
        assert!(xml.contains("--schedule abcdef1234 --if-triggered</Arguments>"));
        assert_eq!(next_after(&s, 0, &fixed), None);
        s.next_ms = 1;
        assert_eq!(due(&s, 1_000_000_000), Due::No);
        assert_eq!(s.describe(), "When a new tag matching v* appears");
    }

    #[test]
    fn task_name_strips_invalid_characters() {
        let s = schedule(Recurrence::Daily);
        assert_eq!(s.task_name(), "Onyx\\My- Game - Nightly (abcdef12)");
    }
}
