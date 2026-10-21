use std::path::Path;
use std::process::Command;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use super::{rel_time, status_color, status_label};
use crate::app::{App, Page};
use crate::model::Project;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind};
use crate::ui::theme;

pub(super) fn show(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) {
    let Some(rec) = app.history_ui.detail.clone() else {
        return;
    };
    let log = app.history_ui.detail_log.clone();
    let px = |v: f32| theme::px(v, s);
    let parsed: Option<Project> = serde_json::from_str(&rec.settings_json).ok();

    let id = ui.make_persistent_id("history_detail");
    let mut open = true;
    let mut rebuild = false;
    let mut delete = false;
    let mut folder = false;
    let mut close = false;

    kit::modal(
        ui,
        id,
        &mut open,
        Vec2::new(px(640.0), px(470.0)),
        s,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(26.0);
            let w = d.width() - px(52.0);
            let painter = ui.painter().clone();
            kit::label(
                &painter,
                Pos2::new(x, d.top() + px(24.0)),
                "BUILD DETAIL",
                s,
            );

            painter.text(
                Pos2::new(d.right() - px(26.0), d.top() + px(28.0)),
                Align2::RIGHT_CENTER,
                format!(
                    "{}  ·  {}",
                    status_label(rec.status),
                    rel_time(rec.timestamp)
                ),
                FontId::new(px(12.5), theme::family_semibold()),
                status_color(rec.status),
            );

            let settings_top = d.top() + px(54.0);
            let settings = Rect::from_min_size(Pos2::new(x, settings_top), Vec2::new(w, px(118.0)));
            kit::bordered(
                &painter,
                settings,
                theme::FIELD,
                theme::LINE,
                px(theme::RADIUS_FIELD),
            );
            let lines = settings_lines(&rec, parsed.as_ref());
            for (i, (k, v)) in lines.iter().enumerate() {
                let y = settings.top() + px(14.0) + i as f32 * px(18.0);
                painter.text(
                    Pos2::new(settings.left() + px(14.0), y),
                    Align2::LEFT_TOP,
                    k,
                    FontId::new(px(11.5), theme::family_label()),
                    theme::INK_MUTED,
                );
                painter.text(
                    Pos2::new(settings.left() + px(140.0), y),
                    Align2::LEFT_TOP,
                    v,
                    FontId::new(px(12.0), theme::family_medium()),
                    theme::INK_DIM,
                );
            }

            let log_top = settings.bottom() + px(14.0);
            let log_rect = Rect::from_min_max(
                Pos2::new(x, log_top),
                Pos2::new(x + w, d.bottom() - px(80.0)),
            );
            kit::bordered(
                &painter,
                log_rect,
                theme::BG,
                theme::LINE,
                px(theme::RADIUS_FIELD),
            );
            let inner = log_rect.shrink(px(12.0));
            let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(inner));
            cu.set_clip_rect(inner);
            egui::ScrollArea::vertical()
                .id_salt("history_detail_scroll")
                .auto_shrink([false, false])
                .show(&mut cu, |ui| {
                    if log.is_empty() {
                        ui.label(
                            egui::RichText::new("Log file is empty or missing.")
                                .font(FontId::new(px(12.0), theme::family_medium()))
                                .color(theme::INK_MUTED),
                        );
                    }
                    for line in log.lines() {
                        ui.label(
                            egui::RichText::new(line)
                                .monospace()
                                .size(px(12.0))
                                .color(theme::INK_DIM),
                        );
                    }
                });

            let by = d.bottom() - px(26.0) - px(40.0);
            let bw = px(150.0);
            if Button::new("OPEN LOG FOLDER", ButtonKind::Secondary)
                .icon(theme::icon::FOLDER)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, by), Vec2::new(px(188.0), px(40.0))),
                    s,
                )
                .clicked()
            {
                folder = true;
            }
            if Button::new("DELETE", ButtonKind::Danger)
                .icon(theme::icon::TRASH)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - px(26.0) - bw * 2.0 - px(10.0), by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked()
            {
                delete = true;
                close = true;
            }
            if Button::new("REBUILD", ButtonKind::Primary)
                .icon(theme::icon::ROCKET)
                .enabled(parsed.is_some())
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
                rebuild = true;
                close = true;
            }
            close || rebuild || delete
        },
    );

    if rebuild {
        if let Some(project) = parsed {
            match app.build_ui.start(&project, &rec.branch, ctx) {
                Ok(()) => {
                    app.nav_request = Some(Page::Build);
                    app.toasts
                        .push(kit::ToastLevel::Success, "Rebuilding with saved settings");
                }
                Err(e) => {
                    app.build_ui
                        .push_log(format!("Rebuild could not start: {e}"), true);
                    app.toasts
                        .push(kit::ToastLevel::Error, "Rebuild failed to start");
                }
            }
        } else {
            app.toasts
                .push(kit::ToastLevel::Error, "Stored settings are unreadable");
        }
        app.history_ui.close_detail();
    } else if delete {
        app.history_ui.delete(&rec.id);
        app.history_ui.close_detail();
        app.toasts
            .push(kit::ToastLevel::Success, "History entry deleted");
    } else if folder {
        open_log_folder(&rec.log_path);
    } else if close || !open {
        app.history_ui.close_detail();
    }
}

fn settings_lines(
    rec: &crate::model::BuildRecord,
    project: Option<&Project>,
) -> Vec<(String, String)> {
    let mut lines = vec![("DESCRIPTION".to_string(), truncate(&rec.description, 64))];
    if let Some(p) = project {
        lines.push(("APP ID".to_string(), p.app_id.clone()));
        lines.push((
            "CONTENT ROOT".to_string(),
            truncate(&p.content_root.to_string_lossy(), 64),
        ));
        lines.push((
            "BUILD OUTPUT".to_string(),
            truncate(&p.build_output_dir.to_string_lossy(), 64),
        ));
        lines.push(("DEPOTS".to_string(), p.depots.len().to_string()));
    }
    lines
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}...", text.chars().take(max).collect::<String>())
    }
}

fn open_log_folder(log_path: &Path) {
    let Some(dir) = log_path.parent() else {
        return;
    };
    #[cfg(windows)]
    let _ = Command::new("explorer").arg(dir).spawn();
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(dir).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(dir).spawn();
}
