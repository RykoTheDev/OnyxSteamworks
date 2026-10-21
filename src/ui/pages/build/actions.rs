use eframe::egui::{self, Pos2, Rect, Vec2};

use crate::app::App;
use crate::features::quick;
use crate::model::Project;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

pub(super) fn open_target(target: String) {
    std::thread::spawn(move || {
        let _ = open::that(target);
    });
}

pub(super) fn quick_actions(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    project: &Project,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let size = rect.height();
    let gap = px(8.0);
    let buttons = [
        theme::icon::PACKAGE,
        theme::icon::PLAY,
        theme::icon::FOLDER,
        theme::icon::DOWNLOAD,
        theme::icon::COPY,
    ];
    let tips = [
        "Steamworks builds",
        "Launch game",
        "Open content root",
        "Open build output",
        "Copy SteamCMD command",
    ];
    for (i, glyph) in buttons.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(rect.left() + i as f32 * (size + gap), rect.top()),
            Vec2::splat(size),
        );
        let resp = kit::icon_button(ui, r, *glyph, s);
        let resp = kit::tooltip(resp, tips[i]);
        if resp.clicked() {
            match i {
                0 => open_target(quick::steamworks_builds_url(&project.app_id)),
                1 => open_target(quick::steam_run_url(&project.app_id)),
                2 => open_target(project.content_root.to_string_lossy().to_string()),
                3 => open_target(project.build_output_dir.to_string_lossy().to_string()),
                _ => {
                    ui.ctx().copy_text(quick::steamcmd_command(project));
                    app.toasts
                        .push(kit::ToastLevel::Success, "SteamCMD command copied");
                }
            }
        }
    }
}

pub(super) fn cli_actions(app: &mut App, ui: &mut egui::Ui, rect: Rect, project: &Project, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let gap = px(10.0);
    let bw = (rect.width() - gap) * 0.5;
    let copy = Rect::from_min_size(rect.min, Vec2::new(bw, rect.height()));
    let export = Rect::from_min_size(
        Pos2::new(copy.right() + gap, rect.top()),
        Vec2::new(bw, rect.height()),
    );

    if Button::new("COPY CLI", ButtonKind::Secondary)
        .icon(theme::icon::COPY)
        .show(ui, copy, s)
        .clicked()
    {
        let branch = project
            .branches
            .get(app.build_ui.branch_sel)
            .map(|b| b.name.clone())
            .unwrap_or_default();
        let command = crate::cli::build_command_string(
            project,
            &branch,
            &app.build_ui.description,
            app.build_ui.preview_only,
            false,
        );
        ui.ctx().copy_text(command);
        app.toasts
            .push(kit::ToastLevel::Success, "CLI command copied");
    }

    if Button::new("EXPORT CI", ButtonKind::Secondary)
        .icon(theme::icon::UPLOAD)
        .show(ui, export, s)
        .clicked()
    {
        let yaml = crate::cli::ci_workflow_yaml(project);
        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("steam-deploy.yml")
                .save_file()
            {
                let _ = std::fs::write(path, yaml);
            }
        });
        app.toasts
            .push(kit::ToastLevel::Success, "Exporting CI workflow");
    }
}
