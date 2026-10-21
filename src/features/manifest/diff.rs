use std::collections::HashMap;

use super::{FileEntry, Manifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffStatus {
    Added,
    Modified,
    Removed,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct DiffEntry {
    pub path: String,
    pub status: DiffStatus,
    pub size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct DiffSummary {
    pub added: usize,
    pub modified: usize,
    pub removed: usize,
    pub unchanged: usize,
    pub added_bytes: u64,
    pub modified_bytes: u64,
    pub removed_bytes: u64,
    pub unchanged_bytes: u64,
    pub entries: Vec<DiffEntry>,
}

impl DiffSummary {
    pub fn changed(&self) -> usize {
        self.added + self.modified + self.removed
    }

    pub fn estimated_upload(&self) -> u64 {
        self.added_bytes + self.modified_bytes
    }

    pub fn largest(&self, n: usize) -> Vec<&DiffEntry> {
        let mut changed: Vec<&DiffEntry> =
            self.entries.iter().filter(|e| e.status != DiffStatus::Unchanged).collect();
        changed.sort_by_key(|e| std::cmp::Reverse(e.size));
        changed.truncate(n);
        changed
    }
}

fn flatten(manifest: &Manifest) -> HashMap<String, &FileEntry> {
    let mut map = HashMap::new();
    for depot in &manifest.depots {
        for file in &depot.files {
            map.insert(format!("{}/{}", depot.depot_id, file.path), file);
        }
    }
    map
}

pub fn diff(current: &Manifest, previous: Option<&Manifest>) -> DiffSummary {
    let cur = flatten(current);
    let prev = previous.map(flatten).unwrap_or_default();
    let mut summary = DiffSummary::default();

    for (key, cf) in &cur {
        let status = match prev.get(key) {
            None => {
                summary.added += 1;
                summary.added_bytes += cf.size;
                DiffStatus::Added
            }
            Some(pf) if pf.hash != cf.hash => {
                summary.modified += 1;
                summary.modified_bytes += cf.size;
                DiffStatus::Modified
            }
            Some(_) => {
                summary.unchanged += 1;
                summary.unchanged_bytes += cf.size;
                DiffStatus::Unchanged
            }
        };
        summary.entries.push(DiffEntry { path: key.clone(), status, size: cf.size });
    }
    for (key, pf) in &prev {
        if !cur.contains_key(key) {
            summary.removed += 1;
            summary.removed_bytes += pf.size;
            summary.entries.push(DiffEntry { path: key.clone(), status: DiffStatus::Removed, size: pf.size });
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::manifest::{DepotManifest, Manifest};

    fn file(path: &str, size: u64, hash: &str) -> FileEntry {
        FileEntry { path: path.to_string(), size, hash: hash.to_string() }
    }

    fn manifest(files: Vec<FileEntry>) -> Manifest {
        Manifest { created_ms: 0, depots: vec![DepotManifest { depot_id: "1001".into(), files }] }
    }

    #[test]
    fn first_build_counts_everything_as_added() {
        let current = manifest(vec![file("a", 10, "h1"), file("b", 20, "h2")]);
        let d = diff(&current, None);
        assert_eq!(d.added, 2);
        assert_eq!(d.modified, 0);
        assert_eq!(d.added_bytes, 30);
        assert_eq!(d.estimated_upload(), 30);
    }

    #[test]
    fn detects_added_modified_removed_unchanged() {
        let prev = manifest(vec![file("keep", 5, "h"), file("change", 8, "old"), file("gone", 4, "g")]);
        let cur = manifest(vec![file("keep", 5, "h"), file("change", 9, "new"), file("fresh", 7, "f")]);
        let d = diff(&cur, Some(&prev));
        assert_eq!(d.unchanged, 1);
        assert_eq!(d.modified, 1);
        assert_eq!(d.added, 1);
        assert_eq!(d.removed, 1);
        assert_eq!(d.modified_bytes, 9);
        assert_eq!(d.added_bytes, 7);
        assert_eq!(d.removed_bytes, 4);
        assert_eq!(d.estimated_upload(), 16);
        assert_eq!(d.changed(), 3);
    }

    #[test]
    fn largest_orders_changed_by_size_and_skips_unchanged() {
        let prev = manifest(vec![file("big", 100, "a")]);
        let cur = manifest(vec![file("big", 100, "a"), file("mid", 50, "m"), file("small", 5, "s")]);
        let d = diff(&cur, Some(&prev));
        let top = d.largest(10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].path, "1001/mid");
        assert_eq!(top[1].path, "1001/small");
    }
}
