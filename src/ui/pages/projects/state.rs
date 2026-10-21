use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use super::scan::{ScanState, run_scan};
use crate::model::{Depot, Project};
use crate::validate::{FieldError, validate_project};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PickField {
    ContentRoot,
    BuildOutput,
    SteamCmd,
    KeyArt,
    DepotLocal,
}

pub struct Pick {
    pub field: PickField,
    pub path: String,
    pub index: usize,
}

#[derive(Clone, Default, PartialEq)]
pub struct Form {
    pub name: String,
    pub app_id: String,
    pub content_root: String,
    pub build_output: String,
    pub steamcmd: String,
    pub username: String,
    pub key_art: String,
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

impl Form {
    pub fn from_project(p: &Project) -> Self {
        Self {
            name: p.name.clone(),
            app_id: p.app_id.clone(),
            content_root: path_str(&p.content_root),
            build_output: path_str(&p.build_output_dir),
            steamcmd: path_str(&p.steamcmd_path),
            username: p.steam_username.clone(),
            key_art: path_str(&p.key_art_path),
        }
    }

    pub fn apply_to(&self, base: &Project) -> Project {
        let mut p = base.clone();
        p.name = self.name.clone();
        p.app_id = self.app_id.clone();
        p.content_root = PathBuf::from(&self.content_root);
        p.build_output_dir = PathBuf::from(&self.build_output);
        p.steamcmd_path = PathBuf::from(&self.steamcmd);
        p.steam_username = self.username.clone();
        p.key_art_path = PathBuf::from(&self.key_art);
        p
    }
}

pub struct ProjectsState {
    pub selected: Option<String>,
    pub base: Project,
    pub form: Form,
    pub on_disk: bool,
    pub errors: Vec<FieldError>,
    pub confirm_delete: bool,
    pub inited: bool,
    pub editor_tab: usize,
    pub vdf_file: usize,
    pub scans: Vec<ScanState>,
    saved_project: Project,
    tx: Sender<Pick>,
    rx: Receiver<Pick>,
    scan_tx: Sender<(usize, ScanState)>,
    scan_rx: Receiver<(usize, ScanState)>,
}

impl ProjectsState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let (scan_tx, scan_rx) = channel();
        Self {
            selected: None,
            base: Project::default(),
            form: Form::default(),
            on_disk: false,
            errors: Vec::new(),
            confirm_delete: false,
            inited: false,
            editor_tab: 0,
            vdf_file: 0,
            scans: Vec::new(),
            saved_project: Project::default(),
            tx,
            rx,
            scan_tx,
            scan_rx,
        }
    }

    pub fn error_for(&self, field: &str) -> Option<String> {
        self.errors
            .iter()
            .find(|e| e.field == field)
            .map(|e| e.message.clone())
    }

    pub fn first_error(&self) -> Option<String> {
        self.errors.first().map(|e| e.message.clone())
    }

    pub fn dirty(&self) -> bool {
        !self.on_disk || self.project() != self.saved_project
    }

    pub fn valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn revalidate(&mut self) {
        self.errors = validate_project(&self.project());
    }

    fn sync_scans(&mut self) {
        self.scans.resize(self.base.depots.len(), ScanState::Idle);
    }

    pub fn select(&mut self, project: &Project) {
        self.selected = Some(project.id.clone());
        self.base = project.clone();
        self.form = Form::from_project(project);
        self.saved_project = project.clone();
        self.on_disk = true;
        self.confirm_delete = false;
        self.scans = vec![ScanState::Idle; project.depots.len()];
        self.revalidate();
    }

    pub fn start_new(&mut self) {
        let project = Project {
            id: uuid::Uuid::new_v4().to_string(),
            ..Default::default()
        };
        self.selected = Some(project.id.clone());
        self.base = project.clone();
        self.form = Form::default();
        self.saved_project = project;
        self.on_disk = false;
        self.confirm_delete = false;
        self.scans.clear();
        self.revalidate();
    }

    pub fn mark_saved(&mut self, project: Project) {
        self.base = project.clone();
        self.saved_project = project;
        self.on_disk = true;
    }

    pub fn project(&self) -> Project {
        self.form.apply_to(&self.base)
    }

    pub fn clear(&mut self) {
        self.selected = None;
        self.base = Project::default();
        self.form = Form::default();
        self.saved_project = Project::default();
        self.on_disk = false;
        self.scans.clear();
        self.revalidate();
    }

    pub fn add_depot(&mut self, depot: Depot) {
        self.base.depots.push(depot);
        self.sync_scans();
        self.revalidate();
    }

    pub fn remove_depot(&mut self, i: usize) {
        if i < self.base.depots.len() {
            self.base.depots.remove(i);
            if i < self.scans.len() {
                self.scans.remove(i);
            }
            self.revalidate();
        }
    }

    pub fn move_depot(&mut self, i: usize, up: bool) {
        let j = if up {
            i.checked_sub(1)
        } else {
            i.checked_add(1).filter(|j| *j < self.base.depots.len())
        };
        if let Some(j) = j {
            self.base.depots.swap(i, j);
            if i < self.scans.len() && j < self.scans.len() {
                self.scans.swap(i, j);
            }
        }
    }

    pub fn add_branch(&mut self) {
        self.base.branches.push(crate::model::Branch::default());
        self.revalidate();
    }

    pub fn remove_branch(&mut self, i: usize) {
        if i < self.base.branches.len() {
            self.base.branches.remove(i);
            self.revalidate();
        }
    }

    pub fn scan_depot(&mut self, ctx: &egui::Context, i: usize) {
        if i >= self.base.depots.len() {
            return;
        }
        self.sync_scans();
        self.scans[i] = ScanState::Running;
        let depot = self.base.depots[i].clone();
        let tx = self.scan_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let (files, bytes) = run_scan(&depot.local_path, &depot.exclusions, depot.recursive);
            let _ = tx.send((i, ScanState::Done { files, bytes }));
            ctx.request_repaint();
        });
    }

    pub fn drain(&mut self) {
        let mut changed = false;
        while let Ok(pick) = self.rx.try_recv() {
            match pick.field {
                PickField::ContentRoot => self.form.content_root = pick.path,
                PickField::BuildOutput => self.form.build_output = pick.path,
                PickField::SteamCmd => self.form.steamcmd = pick.path,
                PickField::KeyArt => self.form.key_art = pick.path,
                PickField::DepotLocal => {
                    if let Some(d) = self.base.depots.get_mut(pick.index) {
                        d.local_path = PathBuf::from(pick.path);
                    }
                }
            }
            changed = true;
        }
        while let Ok((i, state)) = self.scan_rx.try_recv() {
            if i < self.scans.len() {
                self.scans[i] = state;
            }
        }
        if changed {
            self.revalidate();
        }
    }

    pub fn spawn_pick(&self, ctx: &egui::Context, field: PickField, index: usize) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let dialog = rfd::FileDialog::new();
            let picked = match field {
                PickField::SteamCmd => dialog.pick_file(),
                PickField::KeyArt => dialog
                    .add_filter("Image", &["png", "jpg", "jpeg"])
                    .pick_file(),
                _ => dialog.pick_folder(),
            };
            if let Some(path) = picked {
                let _ = tx.send(Pick {
                    field,
                    path: path.to_string_lossy().to_string(),
                    index,
                });
                ctx.request_repaint();
            }
        });
    }
}
