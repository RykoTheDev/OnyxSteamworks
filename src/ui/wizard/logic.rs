use std::path::PathBuf;
use std::sync::mpsc::channel;

use eframe::egui;

use super::{Intent, PickTarget, STEPS};
use crate::app::{App, Page};
use crate::features::importer;
use crate::features::wizard::{self, DownloadMsg};
use crate::model::Project;
use crate::ui::kit;

pub(super) fn pump(app: &mut App, ctx: &egui::Context) {
    while let Ok((target, path)) = app.wizard.pick_rx.try_recv() {
        match target {
            PickTarget::Install => app.wizard.install_dir = path,
            PickTarget::Game => {
                app.wizard.game_dir = path.clone();
                app.wizard.detecting = true;
                app.wizard.detected = None;
                app.wizard.created = None;
                let tx = app.wizard.detect_tx.clone();
                let ctx2 = ctx.clone();
                std::thread::spawn(move || {
                    let detected = importer::detect(&PathBuf::from(path));
                    let _ = tx.send(detected);
                    ctx2.request_repaint();
                });
            }
        }
    }
    while let Ok(d) = app.wizard.detect_rx.try_recv() {
        app.wizard.detected = Some(d);
        app.wizard.detecting = false;
    }

    let msgs: Vec<DownloadMsg> = match &app.wizard.dl_rx {
        Some(rx) => {
            let mut v = Vec::new();
            while let Ok(m) = rx.try_recv() {
                v.push(m);
            }
            v
        }
        None => Vec::new(),
    };
    for msg in msgs {
        match msg {
            DownloadMsg::Progress { done, total } => {
                app.wizard.dl_done = done;
                app.wizard.dl_total = total;
            }
            DownloadMsg::Extracting => app.wizard.extracting = true,
            DownloadMsg::Done(path) => {
                app.wizard.extracting = false;
                let shown = path.to_string_lossy().to_string();
                save_override(&shown);
                app.wizard.installed = Some(shown);
                app.wizard.dl_rx = None;
            }
            DownloadMsg::Error(e) => {
                app.wizard.extracting = false;
                app.wizard.dl_error = Some(e);
                app.wizard.dl_rx = None;
            }
        }
    }
}

pub(super) fn apply(app: &mut App, ui: &egui::Ui, intent: Intent, ctx: &egui::Context) {
    if intent.pick_install {
        spawn_pick(app, PickTarget::Install, ui);
    }
    if intent.pick_game {
        spawn_pick(app, PickTarget::Game, ui);
    }
    if intent.start_install && app.wizard.dl_rx.is_none() {
        let dir = if app.wizard.install_dir.trim().is_empty() {
            default_install_dir()
        } else {
            PathBuf::from(app.wizard.install_dir.trim())
        };
        app.wizard.install_dir = dir.to_string_lossy().to_string();
        app.wizard.dl_error = None;
        app.wizard.dl_done = 0;
        app.wizard.dl_total = 0;
        app.wizard.installed = None;
        let (tx, rx) = channel();
        app.wizard.dl_rx = Some(rx);
        wizard::install_steamcmd(dir, tx, ctx.clone());
    }
    if intent.create_project
        && let Some(d) = app.wizard.detected.clone()
    {
        let project = importer::to_project(&d);
        match crate::config::save_project(&project) {
            Ok(()) => {
                app.projects.push(project.clone());
                app.projects_ui.select(&project);
                app.wizard.created = Some(project.name.clone());
                app.toasts.push(
                    kit::ToastLevel::Success,
                    format!("Created {}", project.name),
                );
            }
            Err(e) => app.toasts.push(
                kit::ToastLevel::Error,
                format!("Could not create project: {e}"),
            ),
        }
    }
    if intent.authorize {
        authorize_account(app, ctx);
    }
    if intent.preview_build {
        if let Some(project) = wizard_project(app)
            && let Some(idx) = app.projects.iter().position(|p| p.id == project.id)
        {
            app.build_ui.project_sel = idx;
        }
        app.build_ui.preview_only = true;
        app.nav_request = Some(Page::Build);
        app.wizard.open = false;
    }

    if (intent.next || intent.skip) && app.wizard.step + 1 < STEPS {
        app.wizard.step += 1;
    }
    if intent.back && app.wizard.step > 0 {
        app.wizard.step -= 1;
    }
    if intent.close {
        app.wizard.open = false;
    }
}

fn authorize_account(app: &mut App, ctx: &egui::Context) {
    let Some(project) = wizard_project(app) else {
        app.toasts.push(
            kit::ToastLevel::Warning,
            "Create a project first, then authorize an account",
        );
        return;
    };
    match app.build_ui.authorize(&project, ctx) {
        Ok(()) => {
            app.wizard.authorized = true;
            if let Some(idx) = app.projects.iter().position(|p| p.id == project.id) {
                app.build_ui.project_sel = idx;
            }
            app.nav_request = Some(Page::Build);
            app.wizard.open = false;
            app.toasts.push(
                kit::ToastLevel::Success,
                "Authorizing — answer any prompt on the Build page",
            );
        }
        Err(e) => app
            .toasts
            .push(kit::ToastLevel::Error, format!("Could not authorize: {e}")),
    }
}

fn spawn_pick(app: &App, target: PickTarget, ui: &egui::Ui) {
    let tx = app.wizard.pick_tx.clone();
    let ctx = ui.ctx().clone();
    std::thread::spawn(move || {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            let _ = tx.send((target, dir.to_string_lossy().to_string()));
            ctx.request_repaint();
        }
    });
}

fn default_install_dir() -> PathBuf {
    crate::config::data_dir()
        .map(|d| d.join("steamcmd"))
        .unwrap_or_else(|_| PathBuf::from("steamcmd"))
}

fn save_override(path: &str) {
    let mut cfg = crate::config::load_settings().unwrap_or_default();
    cfg.steamcmd_path = path.to_string();
    if let Err(e) = crate::config::save_settings(&cfg) {
        eprintln!("could not save steamcmd path: {e}");
    }
}

pub(super) fn mark_done() {
    let mut cfg = crate::config::load_settings().unwrap_or_default();
    if !cfg.wizard_done {
        cfg.wizard_done = true;
        if let Err(e) = crate::config::save_settings(&cfg) {
            eprintln!("could not save wizard state: {e}");
        }
    }
}

pub(super) fn wizard_project(app: &App) -> Option<Project> {
    if let Some(name) = &app.wizard.created
        && let Some(p) = app.projects.iter().find(|p| &p.name == name)
    {
        return Some(p.clone());
    }
    app.projects
        .get(app.build_ui.project_sel)
        .cloned()
        .or_else(|| app.projects.first().cloned())
}

pub(super) fn wizard_account(app: &App) -> String {
    wizard_project(app)
        .map(|p| {
            if p.default_account.is_empty() {
                p.steam_username
            } else {
                p.default_account
            }
        })
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| "No account set yet".to_string())
}
