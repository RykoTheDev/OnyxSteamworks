use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use eframe::egui::{Pos2, Rect, Vec2};

use super::Action;
use crate::app::App;
use crate::config;
use crate::model::Project;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

pub enum IoResult {
    Imported(Box<Project>),
    Exported,
    Error(String),
}

pub struct ProjectIo {
    tx: Sender<IoResult>,
    rx: Receiver<IoResult>,
}

impl Default for ProjectIo {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx }
    }
}

impl ProjectIo {
    pub fn drain(&self) -> Vec<IoResult> {
        let mut out = Vec::new();
        while let Ok(res) = self.rx.try_recv() {
            out.push(res);
        }
        out
    }

    pub fn spawn_import(&self, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let picked = rfd::FileDialog::new()
                .add_filter("Steam VDF", &["vdf"])
                .add_filter("Project JSON", &["json"])
                .pick_file();
            if let Some(path) = picked {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let result = if ext == "json" {
                    std::fs::read_to_string(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|t| {
                            crate::features::profile::import_json(&t).map_err(|e| e.to_string())
                        })
                } else {
                    crate::features::import_vdf::import_appbuild(&path).map_err(|e| e.to_string())
                };
                let _ = tx.send(match result {
                    Ok(project) => IoResult::Imported(Box::new(project)),
                    Err(e) => IoResult::Error(e),
                });
                ctx.request_repaint();
            }
        });
    }

    pub fn spawn_export(&self, project: Project, ctx: &egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let name = if project.name.is_empty() {
                "project".to_string()
            } else {
                project.name.clone()
            };
            let picked = rfd::FileDialog::new()
                .add_filter("Project JSON", &["json"])
                .set_file_name(format!("{name}.json"))
                .save_file();
            if let Some(path) = picked {
                let result = crate::features::profile::export_json(&project)
                    .map_err(|e| e.to_string())
                    .and_then(|json| std::fs::write(&path, json).map_err(|e| e.to_string()));
                let _ = tx.send(match result {
                    Ok(()) => IoResult::Exported,
                    Err(e) => IoResult::Error(e),
                });
                ctx.request_repaint();
            }
        });
    }
}

pub(super) fn toolbar(
    app: &mut App,
    ui: &mut eframe::egui::Ui,
    rect: Rect,
    s: f32,
) -> (Option<Action>, f32) {
    let px = |v: f32| theme::px(v, s);
    let row_y = rect.top() + px(30.0);
    let bw = (rect.width() - px(16.0)) / 3.0;
    let has_sel = app.projects_ui.selected.is_some() && app.projects_ui.on_disk;
    let mut action = None;

    if Button::new("NEW", ButtonKind::Primary)
        .icon(theme::icon::PLUS)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(rect.left(), row_y), Vec2::new(bw, px(36.0))),
            s,
        )
        .clicked()
    {
        action = Some(Action::New);
    }
    if Button::new("DUP", ButtonKind::Secondary)
        .enabled(has_sel)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.left() + bw + px(8.0), row_y),
                Vec2::new(bw, px(36.0)),
            ),
            s,
        )
        .clicked()
        && has_sel
    {
        action = Some(Action::Duplicate);
    }
    if Button::new("DEL", ButtonKind::Danger)
        .enabled(has_sel)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.left() + (bw + px(8.0)) * 2.0, row_y),
                Vec2::new(bw, px(36.0)),
            ),
            s,
        )
        .clicked()
        && has_sel
    {
        action = Some(Action::Delete);
    }

    let io_y = row_y + px(44.0);
    let iw = (rect.width() - px(8.0)) / 2.0;
    if Button::new("IMPORT", ButtonKind::Secondary)
        .icon(theme::icon::DOWNLOAD)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(rect.left(), io_y), Vec2::new(iw, px(34.0))),
            s,
        )
        .clicked()
    {
        action = Some(Action::Import);
    }
    if Button::new("EXPORT", ButtonKind::Secondary)
        .icon(theme::icon::UPLOAD)
        .enabled(has_sel)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.left() + iw + px(8.0), io_y),
                Vec2::new(iw, px(34.0)),
            ),
            s,
        )
        .clicked()
        && has_sel
    {
        action = Some(Action::Export);
    }
    (action, io_y + px(46.0))
}

pub(super) fn handle(app: &mut App, res: IoResult) {
    match res {
        IoResult::Imported(project) => {
            let project = *project;
            match config::save_project(&project) {
                Ok(()) => {
                    super::reload(app);
                    app.projects_ui.select(&project);
                    let name = if project.name.is_empty() {
                        "project".to_string()
                    } else {
                        project.name.clone()
                    };
                    app.toasts
                        .push(kit::ToastLevel::Success, format!("Imported {name}"));
                }
                Err(e) => app
                    .toasts
                    .push(kit::ToastLevel::Error, format!("Import failed: {e}")),
            }
        }
        IoResult::Exported => app
            .toasts
            .push(kit::ToastLevel::Success, "Project exported"),
        IoResult::Error(e) => app.toasts.push(kit::ToastLevel::Error, e),
    }
}
