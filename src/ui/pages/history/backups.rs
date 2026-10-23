use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, UiBuilder, Vec2};

use crate::app::App;
use crate::features::backup::{self, BackupInfo, Progress};
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;
use crate::validate::human_size;

enum Msg {
    Folder(PathBuf),
    Backed(Result<BackupInfo, String>),
    Restored(Result<(), String>),
}

pub struct BackupState {
    project_sel: usize,
    infos: Vec<BackupInfo>,
    loaded_key: Option<String>,
    running: bool,
    progress: Arc<Progress>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    confirm_delete: Option<PathBuf>,
}

impl BackupState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            project_sel: 0,
            infos: Vec::new(),
            loaded_key: None,
            running: false,
            progress: Arc::new(Progress::default()),
            tx,
            rx,
            confirm_delete: None,
        }
    }
}

fn settings() -> crate::config::Settings {
    crate::config::load_settings().unwrap_or_default()
}

fn save(settings: &crate::config::Settings) {
    if let Err(e) = crate::config::save_settings(settings) {
        eprintln!("could not save settings: {e}");
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    drain(app, &ctx);

    let mut cfg = settings();
    if cfg.backup_keep == 0 {
        cfg.backup_keep = 5;
    }
    if app.backup_ui.project_sel >= app.projects.len() {
        app.backup_ui.project_sel = 0;
    }
    let project = app.projects.get(app.backup_ui.project_sel).cloned();
    let key = format!(
        "{}|{}",
        cfg.backup_dir,
        project.as_ref().map(|p| p.id.as_str()).unwrap_or("")
    );
    if app.backup_ui.loaded_key.as_deref() != Some(key.as_str()) {
        app.backup_ui.infos = project
            .as_ref()
            .filter(|_| !cfg.backup_dir.is_empty())
            .map(|p| backup::list_backups(&PathBuf::from(&cfg.backup_dir), &p.id))
            .unwrap_or_default();
        app.backup_ui.loaded_key = Some(key);
    }

    controls(app, ui, rect, &mut cfg, s);

    let list = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + px(118.0)), rect.max);
    if app.backup_ui.infos.is_empty() {
        kit::empty_state(
            ui,
            list,
            theme::icon::PACKAGE,
            "No backups yet",
            "Run a pipeline Backup step or use Back up now.",
            s,
        );
    } else {
        rows(app, ui, list, s);
    }
    delete_modal(app, ui, s);
}

fn controls(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    cfg: &mut crate::config::Settings,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let folder = if cfg.backup_dir.is_empty() {
        "No backup folder set".to_string()
    } else {
        cfg.backup_dir.clone()
    };
    ui.painter().text(
        Pos2::new(rect.left(), rect.top() + px(12.0)),
        Align2::LEFT_CENTER,
        folder,
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_DIM,
    );
    if Button::new("Choose folder", ButtonKind::Secondary)
        .icon(theme::icon::FOLDER)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.right() - px(160.0), rect.top()),
                Vec2::new(px(160.0), px(34.0)),
            ),
            s,
        )
        .clicked()
    {
        let tx = app.backup_ui.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                let _ = tx.send(Msg::Folder(dir));
                ctx.request_repaint();
            }
        });
    }

    let y2 = rect.top() + px(46.0);
    ui.painter().text(
        Pos2::new(rect.left(), y2 + px(15.0)),
        Align2::LEFT_CENTER,
        format!("Keep last {}", cfg.backup_keep),
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_DIM,
    );
    let minus = Rect::from_min_size(
        Pos2::new(rect.left() + px(110.0), y2),
        Vec2::splat(px(30.0)),
    );
    let plus = Rect::from_min_size(
        Pos2::new(minus.right() + px(6.0), y2),
        Vec2::splat(px(30.0)),
    );
    if Button::new("-", ButtonKind::Secondary)
        .show(ui, minus, s)
        .clicked()
        && cfg.backup_keep > 1
    {
        cfg.backup_keep -= 1;
        save(cfg);
    }
    if Button::new("+", ButtonKind::Secondary)
        .show(ui, plus, s)
        .clicked()
        && cfg.backup_keep < 50
    {
        cfg.backup_keep += 1;
        save(cfg);
    }
    let tog = Rect::from_min_size(
        Pos2::new(plus.right() + px(24.0), y2 + px(3.0)),
        Vec2::new(px(44.0), px(24.0)),
    );
    let mut prune = cfg.backup_auto_prune;
    if kit::toggle(ui, tog, &mut prune, s).changed() {
        cfg.backup_auto_prune = prune;
        save(cfg);
    }
    ui.painter().text(
        Pos2::new(tog.right() + px(10.0), tog.center().y),
        Align2::LEFT_CENTER,
        "Auto-delete old backups",
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_DIM,
    );

    let can_backup = !app.backup_ui.running
        && !cfg.backup_dir.is_empty()
        && app
            .projects
            .get(app.backup_ui.project_sel)
            .map(|p| p.content_root.is_dir())
            .unwrap_or(false);
    if Button::new("Back up now", ButtonKind::Primary)
        .icon(theme::icon::PACKAGE)
        .enabled(can_backup)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.right() - px(160.0), y2),
                Vec2::new(px(160.0), px(34.0)),
            ),
            s,
        )
        .clicked()
        && can_backup
    {
        start_backup(app, cfg, &ctx);
    }
    if app.backup_ui.running {
        let p = &app.backup_ui.progress;
        let frac =
            p.done.load(Ordering::Relaxed) as f32 / p.total.load(Ordering::Relaxed).max(1) as f32;
        kit::progress_bar(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.left(), rect.top() + px(92.0)),
                Vec2::new(rect.width(), px(5.0)),
            ),
            frac,
            s,
        );
        ctx.request_repaint();
    } else {
        project_chips(app, ui, rect, rect.top() + px(86.0), s);
    }
}

fn project_chips(app: &mut App, ui: &mut egui::Ui, rect: Rect, y: f32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let mut x = rect.left();
    for (i, p) in app.projects.iter().enumerate() {
        let label = if p.name.is_empty() {
            "Untitled".to_string()
        } else {
            p.name.clone()
        };
        let size = kit::chip_size(ui, &label, s);
        if x + size.x > rect.right() {
            break;
        }
        if kit::chip(
            ui,
            Rect::from_min_size(Pos2::new(x, y), size),
            &label,
            i == app.backup_ui.project_sel,
            s,
        )
        .clicked()
        {
            app.backup_ui.project_sel = i;
            app.backup_ui.loaded_key = None;
        }
        x += size.x + px(8.0);
    }
}

fn start_backup(app: &mut App, cfg: &crate::config::Settings, ctx: &egui::Context) {
    let Some(project) = app.projects.get(app.backup_ui.project_sel).cloned() else {
        return;
    };
    let root = PathBuf::from(&cfg.backup_dir);
    let number = backup::next_build_number(&root, &project.id);
    let keep = cfg.backup_keep.max(1);
    let prune = cfg.backup_auto_prune;
    app.backup_ui.progress = Arc::new(Progress::default());
    app.backup_ui.running = true;
    let progress = app.backup_ui.progress.clone();
    let tx = app.backup_ui.tx.clone();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let result = backup::create_backup(&project, &root, number, keep, prune, Some(&progress))
            .map_err(|e| e.to_string());
        let _ = tx.send(Msg::Backed(result));
        ctx.request_repaint();
    });
}

fn rows(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    use super::rel_time;
    let px = |v: f32| theme::px(v, s);
    let infos = app.backup_ui.infos.clone();
    let mut reveal = None;
    let mut restore = None;
    let mut delete = None;
    let mut cu = ui.new_child(UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("backups_list")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            ui.set_width(rect.width() - px(14.0));
            for info in &infos {
                let (r, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), px(52.0)),
                    egui::Sense::hover(),
                );
                kit::bordered(
                    ui.painter(),
                    r,
                    theme::SURFACE_2,
                    theme::LINE,
                    px(theme::RADIUS_CARD),
                );
                ui.painter().text(
                    Pos2::new(r.left() + px(16.0), r.center().y - px(8.0)),
                    Align2::LEFT_CENTER,
                    format!("Build #{}", info.meta.build_number),
                    FontId::new(px(13.5), theme::family_semibold()),
                    theme::INK,
                );
                let sub = format!(
                    "{}  ·  {}",
                    human_size(info.size),
                    rel_time(info.meta.created_ms)
                );
                ui.painter().text(
                    Pos2::new(r.left() + px(16.0), r.center().y + px(9.0)),
                    Align2::LEFT_CENTER,
                    sub,
                    FontId::new(px(11.5), theme::family_medium()),
                    theme::INK_MUTED,
                );
                let bw = px(30.0);
                let dz = Rect::from_min_size(
                    Pos2::new(r.right() - px(10.0) - bw, r.center().y - bw / 2.0),
                    Vec2::splat(bw),
                );
                let rz = Rect::from_min_size(
                    Pos2::new(dz.left() - px(6.0) - bw, r.center().y - bw / 2.0),
                    Vec2::splat(bw),
                );
                let vz = Rect::from_min_size(
                    Pos2::new(rz.left() - px(6.0) - bw, r.center().y - bw / 2.0),
                    Vec2::splat(bw),
                );
                if kit::icon_button(ui, vz, theme::icon::FOLDER, s).clicked() {
                    reveal = info.path.parent().map(|p| p.to_path_buf());
                }
                if kit::icon_button(ui, rz, theme::icon::DOWNLOAD, s).clicked() {
                    restore = Some(info.path.clone());
                }
                if kit::icon_button(ui, dz, theme::icon::TRASH, s).clicked() {
                    delete = Some(info.path.clone());
                }
                ui.add_space(px(10.0));
            }
        });
    if let Some(dir) = reveal {
        std::thread::spawn(move || {
            let _ = open::that(dir);
        });
    }
    if let Some(zip) = restore {
        start_restore(app, zip, &ui.ctx().clone());
    }
    if let Some(zip) = delete {
        app.backup_ui.confirm_delete = Some(zip);
    }
}

fn start_restore(app: &mut App, zip: PathBuf, ctx: &egui::Context) {
    let tx = app.backup_ui.tx.clone();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        if let Some(dest) = rfd::FileDialog::new().pick_folder() {
            let result = backup::restore_backup(&zip, &dest).map_err(|e| e.to_string());
            let _ = tx.send(Msg::Restored(result));
            ctx.request_repaint();
        }
    });
}

fn delete_modal(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let Some(path) = app.backup_ui.confirm_delete.clone() else {
        return;
    };
    let id = ui.make_persistent_id("backup_delete_modal");
    let mut open = true;
    let mut act = 0;
    kit::modal(
        ui,
        id,
        &mut open,
        Vec2::new(px(420.0), px(180.0)),
        s,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(26.0);
            kit::label(
                &ui.painter().clone(),
                Pos2::new(x, d.top() + px(26.0)),
                "Delete backup",
                s,
            );
            ui.painter().text(
                Pos2::new(x, d.top() + px(58.0)),
                Align2::LEFT_TOP,
                "This permanently deletes the backup zip.",
                FontId::new(px(12.5), theme::family_medium()),
                theme::INK_DIM,
            );
            let bw = px(150.0);
            let by = d.bottom() - px(26.0) - px(40.0);
            if Button::new("Cancel", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, by), Vec2::new(bw, px(40.0))),
                    s,
                )
                .clicked()
            {
                act = 1;
            }
            if Button::new("Delete", ButtonKind::Danger)
                .icon(theme::icon::TRASH)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - px(26.0) - bw, by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked()
            {
                act = 2;
            }
            act != 0
        },
    );
    if act == 2 {
        if let Err(e) = backup::delete_backup(&path) {
            app.toasts
                .push(kit::ToastLevel::Error, format!("Delete failed: {e}"));
        } else {
            app.toasts.push(kit::ToastLevel::Success, "Backup deleted");
        }
        app.backup_ui.loaded_key = None;
        app.backup_ui.confirm_delete = None;
    } else if act == 1 || !open {
        app.backup_ui.confirm_delete = None;
    }
}

fn drain(app: &mut App, ctx: &egui::Context) {
    while let Ok(msg) = app.backup_ui.rx.try_recv() {
        match msg {
            Msg::Folder(dir) => {
                let mut cfg = settings();
                cfg.backup_dir = dir.to_string_lossy().to_string();
                save(&cfg);
                app.backup_ui.loaded_key = None;
            }
            Msg::Backed(result) => {
                app.backup_ui.running = false;
                app.backup_ui.loaded_key = None;
                match result {
                    Ok(info) => app.toasts.push(
                        kit::ToastLevel::Success,
                        format!("Backed up ({})", human_size(info.size)),
                    ),
                    Err(e) => app
                        .toasts
                        .push(kit::ToastLevel::Error, format!("Backup failed: {e}")),
                }
            }
            Msg::Restored(result) => match result {
                Ok(()) => app.toasts.push(kit::ToastLevel::Success, "Backup restored"),
                Err(e) => app
                    .toasts
                    .push(kit::ToastLevel::Error, format!("Restore failed: {e}")),
            },
        }
    }
    if app.backup_ui.running {
        ctx.request_repaint();
    }
}
