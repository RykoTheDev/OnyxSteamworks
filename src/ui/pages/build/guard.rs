use eframe::egui::{self, FontId, Key, Pos2, Rect, Vec2};

use crate::app::App;
use crate::model::{BranchLevel, Project};
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::theme;

pub struct GuardState {
    open: bool,
    branch: String,
    typed: String,
}

impl GuardState {
    pub fn new() -> Self {
        Self {
            open: false,
            branch: String::new(),
            typed: String::new(),
        }
    }
}

pub(crate) fn is_live(project: &Project, branch: &str) -> bool {
    project
        .branches
        .iter()
        .find(|b| b.name == branch)
        .map(|b| b.level == BranchLevel::Live)
        .unwrap_or(false)
}

pub(crate) fn request(app: &mut App, project: &Project, branch: &str, ctx: &egui::Context) {
    if is_live(project, branch) {
        app.build_ui.guard.open = true;
        app.build_ui.guard.branch = branch.to_string();
        app.build_ui.guard.typed.clear();
    } else {
        app.build_ui.preflight.request(project, branch, ctx);
    }
}

pub(crate) fn show(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id("live_guard_modal");
    let mut open = app.build_ui.guard.open;
    let branch = app.build_ui.guard.branch.clone();
    let matches = app.build_ui.guard.typed == branch && !branch.is_empty();
    let mut confirm = false;
    let mut cancel = false;
    let typed = &mut app.build_ui.guard.typed;

    kit::modal(
        ui,
        id,
        &mut open,
        Vec2::new(px(460.0), px(248.0)),
        s,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(26.0);
            kit::label(
                &ui.painter().clone(),
                Pos2::new(x, d.top() + px(26.0)),
                "CONFIRM LIVE BUILD",
                s,
            );
            let note = format!(
                "This builds to the LIVE branch \"{branch}\". Type the branch name to confirm."
            );
            let galley = ui.painter().layout(
                note,
                FontId::new(px(12.5), theme::family_medium()),
                theme::DANGER,
                d.width() - px(52.0),
            );
            ui.painter()
                .galley(Pos2::new(x, d.top() + px(52.0)), galley, theme::DANGER);

            let field = Rect::from_min_size(
                Pos2::new(x, d.bottom() - px(118.0)),
                Vec2::new(d.width() - px(52.0), px(40.0)),
            );
            let resp = Input::new(typed).hint(&branch).show(ui, field, s);
            let entered = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));

            let bw = px(150.0);
            let by = d.bottom() - px(26.0) - px(40.0);
            if Button::new("CANCEL", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, by), Vec2::new(bw, px(40.0))),
                    s,
                )
                .clicked()
            {
                cancel = true;
            }
            let go = Button::new("BUILD TO LIVE", ButtonKind::Danger)
                .enabled(matches)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - px(26.0) - bw, by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked();
            if (go || (entered && matches)) && matches {
                confirm = true;
            }
            confirm || cancel
        },
    );

    if confirm {
        app.build_ui.guard.open = false;
        let sel = app.build_ui.project_sel;
        if let Some(project) = app.projects.get(sel).cloned() {
            app.build_ui.preflight.request(&project, &branch, ctx);
        }
    } else if cancel {
        app.build_ui.guard.open = false;
    } else {
        app.build_ui.guard.open = open;
    }
}
