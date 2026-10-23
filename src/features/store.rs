use std::path::PathBuf;
use std::time::SystemTime;

use serde::Serialize;
use serde::de::DeserializeOwned;

fn path(name: &str) -> Option<PathBuf> {
    crate::config::data_dir().ok().map(|d| d.join(name))
}

pub fn stamp(name: &str) -> Option<SystemTime> {
    path(name)
        .and_then(|p| std::fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
}

pub fn load<T: DeserializeOwned>(name: &str) -> Vec<T> {
    path(name)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save<T: Serialize>(name: &str, items: &[T]) {
    let Some(path) = path(name) else {
        return;
    };
    let Ok(bytes) = serde_json::to_vec_pretty(items) else {
        return;
    };
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, bytes).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}
