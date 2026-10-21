use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;
use walkdir::WalkDir;

use crate::model::{BuildRecord, Project};

pub struct HomeState {
    sizes: HashMap<String, u64>,
    requested: HashSet<String>,
    tx: Sender<(String, u64)>,
    rx: Receiver<(String, u64)>,
}

impl HomeState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { sizes: HashMap::new(), requested: HashSet::new(), tx, rx }
    }

    pub fn size_label(&self, id: &str) -> Option<String> {
        self.sizes.get(id).map(|b| crate::validate::human_size(*b))
    }

    pub fn ensure(&mut self, records: &[&BuildRecord], ctx: &egui::Context) {
        while let Ok((id, bytes)) = self.rx.try_recv() {
            self.sizes.insert(id, bytes);
        }
        for record in records {
            if self.requested.contains(&record.id) {
                continue;
            }
            self.requested.insert(record.id.clone());
            let id = record.id.clone();
            let json = record.settings_json.clone();
            let tx = self.tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send((id, upload_size(&json)));
                ctx.request_repaint();
            });
        }
    }
}

fn upload_size(json: &str) -> u64 {
    let Ok(project) = serde_json::from_str::<Project>(json) else {
        return 0;
    };
    let mut total = 0u64;
    for depot in &project.depots {
        if depot.local_path.as_os_str().is_empty() {
            continue;
        }
        let depth = if depot.recursive { usize::MAX } else { 1 };
        for entry in WalkDir::new(&depot.local_path).max_depth(depth).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}
