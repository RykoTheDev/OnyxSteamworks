use std::path::Path;

use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ScanState {
    #[default]
    Idle,
    Running,
    Done {
        files: u64,
        bytes: u64,
    },
}

fn normalize(pattern: &str) -> String {
    if pattern.starts_with('.') && !pattern.contains('*') && !pattern.contains('/') {
        format!("*{pattern}")
    } else {
        pattern.to_string()
    }
}

pub fn run_scan(root: &Path, exclusions: &[String], recursive: bool) -> (u64, u64) {
    let mut builder = GlobSetBuilder::new();
    for pat in exclusions {
        let pat = pat.trim();
        if pat.is_empty() {
            continue;
        }
        if let Ok(glob) = Glob::new(&normalize(pat)) {
            builder.add(glob);
        }
    }
    let set = builder
        .build()
        .unwrap_or_else(|_| globset::GlobSet::empty());

    let max_depth = if recursive { usize::MAX } else { 1 };
    let mut files = 0u64;
    let mut bytes = 0u64;
    for entry in WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if set.is_match(name.as_ref()) {
            continue;
        }
        files += 1;
        bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
    }
    (files, bytes)
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}
