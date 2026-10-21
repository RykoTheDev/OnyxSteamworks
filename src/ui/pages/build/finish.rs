use eframe::egui;

use crate::app::App;
use crate::model::{BranchLevel, BuildRecord, BuildStatus};
use crate::ui::kit;

pub(super) fn after_finish(app: &mut App, record: &BuildRecord, ctx: &egui::Context) {
    maybe_reset_checklist(app, record);
    fire_notification(app, record, ctx);
}

fn maybe_reset_checklist(app: &mut App, record: &BuildRecord) {
    if record.status != BuildStatus::Success {
        return;
    }
    if !crate::config::load_settings()
        .map(|c| c.checklist_reset_on_live)
        .unwrap_or(false)
    {
        return;
    }
    let Some(idx) = app.projects.iter().position(|p| p.id == record.project_id) else {
        return;
    };
    let is_live = app.projects[idx]
        .branches
        .iter()
        .find(|b| b.name == record.branch)
        .map(|b| b.level == BranchLevel::Live)
        .unwrap_or(false);
    if !is_live {
        return;
    }
    crate::features::checklist::reset(&mut app.projects[idx].checklist);
    let project = app.projects[idx].clone();
    let _ = crate::config::save_project(&project);
    app.toasts.push(
        kit::ToastLevel::Success,
        "Release checklist reset for the next build",
    );
}

fn fire_notification(app: &mut App, record: &BuildRecord, ctx: &egui::Context) {
    use crate::features::notify::{BuildInfo, Outcome};
    let Some(outcome) = Outcome::from_status(record.status) else {
        return;
    };
    let project = app
        .projects
        .iter()
        .find(|p| p.id == record.project_id)
        .map(|p| {
            if p.name.is_empty() {
                "Untitled".to_string()
            } else {
                p.name.clone()
            }
        })
        .unwrap_or_else(|| "Project".to_string());
    let number = (app.history_ui.for_project(&record.project_id).len() + 1) as u32;
    let info = BuildInfo {
        outcome,
        project,
        number,
        duration_ms: record.duration_ms,
        branch: record.branch.clone(),
        notes: record.notes.clone(),
    };
    app.notify_ui.fire(info, ctx);
}
