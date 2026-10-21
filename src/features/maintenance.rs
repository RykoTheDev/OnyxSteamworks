use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::model::Project;

pub fn effective_steamcmd(project: &Project) -> PathBuf {
    let settings = crate::config::load_settings().unwrap_or_default();
    if settings.use_fake_steamcmd
        && let Ok(mut exe) = std::env::current_exe()
    {
        exe.pop();
        if exe.file_name().is_some_and(|n| n == "deps") {
            exe.pop();
        }
        exe.push(format!("fake_steamcmd{}", std::env::consts::EXE_SUFFIX));
        return exe;
    }
    let over = settings.steamcmd_override.trim();
    if !over.is_empty() {
        return PathBuf::from(over);
    }
    project.steamcmd_path.clone()
}

pub fn clean_old_logs(days: u32) -> usize {
    if days == 0 {
        return 0;
    }
    let Ok(dir) = crate::config::log_dir() else {
        return 0;
    };
    let cutoff = Duration::from_secs(days as u64 * 86_400);
    let now = SystemTime::now();
    let mut removed = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let aged = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| now.duration_since(t).ok())
                .is_some_and(|age| age > cutoff);
            if aged && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}
