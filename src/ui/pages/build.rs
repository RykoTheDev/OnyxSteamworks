mod actions;
mod auth;
mod console;
mod guard;
mod launch;
pub(crate) mod preflight;
pub(crate) mod review;
mod speed;
mod stages;
mod state;
mod verify;

pub use state::BuildState;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, UiBuilder, Vec2};

use crate::app::{App, Page};
use crate::model::{BranchLevel, Project};
use crate::steamcmd::BuildResult;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::layout::Layout;
use crate::ui::{anim, theme};
use crate::validate::validate_project;

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let ctx = ui.ctx().clone();
    let s = layout.scale;
    let px = |v: f32| theme::px(v, s);
    let content = layout.content;

    if let Some(res) = app.build_ui.drain(&ctx) {
        let (level, msg) = match res {
            BuildResult::Success { .. } => (kit::ToastLevel::Success, "Build finished"),
            BuildResult::Failure { .. } => (kit::ToastLevel::Error, "Build failed"),
            BuildResult::Cancelled => (kit::ToastLevel::Warning, "Build cancelled"),
        };
        app.toasts.push(level, msg);
    }
    if let Some(record) = app.build_ui.finished_record.take() {
        app.history_ui.save(record);
    }
    app.history_ui.ensure_loaded();

    if !app.build_ui.inited {
        app.build_ui.project_sel = app
            .projects_ui
            .selected
            .as_ref()
            .and_then(|id| app.projects.iter().position(|p| &p.id == id))
            .unwrap_or(0);
        if let Some(p) = app.projects.get(app.build_ui.project_sel)
            && app.history_ui.latest_for(&p.id).is_none()
        {
            app.build_ui.preview_only = true;
        }
        app.build_ui.inited = true;
    }

    kit::hero(&ui.painter().clone(), content, "BUILD", s);
    let body_top = content.top() + px(96.0);

    if app.projects.is_empty() {
        let body = Rect::from_min_max(Pos2::new(content.left(), body_top), content.max);
        kit::empty_state(
            ui,
            body,
            theme::icon::ROCKET,
            "No projects to build",
            "Create a project first, then run a build here.",
            s,
        );
        return;
    }
    app.build_ui.project_sel = app.build_ui.project_sel.min(app.projects.len() - 1);

    let right_w = px(312.0);
    let gap = px(20.0);
    let right_x = content.right() - right_w;
    let left = Rect::from_min_max(
        Pos2::new(content.left(), body_top),
        Pos2::new(right_x - gap, content.bottom()),
    );
    let right = Rect::from_min_max(Pos2::new(right_x, body_top), content.max);

    let project = app.projects[app.build_ui.project_sel].clone();
    left_panel(app, ui, left, &project, s);
    right_column(app, ui, right, &project, s, &ctx);
    auth::show(app, ui, s);
    guard::show(app, ui, s, &ctx);
    preflight::show(app, ui, s, &ctx);
    review::show(app, ui, s, &ctx);
    verify::show(app, ui, s, &ctx);
}

fn left_panel(app: &mut App, ui: &mut egui::Ui, rect: Rect, project: &Project, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    let pad = px(18.0);
    let inner_x = rect.left() + pad;
    let inner_w = rect.width() - pad * 2.0;

    let dd_rect = Rect::from_min_size(
        Pos2::new(inner_x, rect.top() + pad),
        Vec2::new(px(240.0), px(40.0)),
    );
    let names: Vec<String> = app
        .projects
        .iter()
        .map(|p| {
            if p.name.is_empty() {
                "Untitled".to_string()
            } else {
                p.name.clone()
            }
        })
        .collect();
    let refs: Vec<&str> = names.iter().map(|x| x.as_str()).collect();
    let prev = app.build_ui.project_sel;
    kit::dropdown(ui, dd_rect, &refs, &mut app.build_ui.project_sel, s);
    if app.build_ui.project_sel != prev {
        app.build_ui.branch_sel = 0;
        if let Some(p) = app.projects.get(app.build_ui.project_sel)
            && app.history_ui.latest_for(&p.id).is_none()
        {
            app.build_ui.preview_only = true;
        }
    }

    let mut x = dd_rect.right() + px(14.0);
    let chip_y = dd_rect.center().y - px(15.0);
    if project.branches.is_empty() {
        ui.painter().text(
            Pos2::new(x, dd_rect.center().y),
            Align2::LEFT_CENTER,
            "default branch",
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
    } else {
        for (i, b) in project.branches.iter().enumerate() {
            let label = if b.name.is_empty() {
                "branch"
            } else {
                b.name.as_str()
            };
            let size = kit::chip_size(ui, label, s);
            let r = Rect::from_min_size(Pos2::new(x, chip_y), size);
            if r.right() > rect.right() - pad {
                break;
            }
            if kit::chip(ui, r, label, i == app.build_ui.branch_sel, s).clicked() {
                app.build_ui.branch_sel = i;
            }
            if b.level == BranchLevel::Live {
                ui.painter().circle_filled(
                    Pos2::new(r.right() - px(3.0), r.top() + px(3.0)),
                    px(3.5),
                    theme::DANGER,
                );
            }
            x += size.x + px(8.0);
        }
    }

    let quick = Rect::from_min_size(
        Pos2::new(inner_x, dd_rect.bottom() + px(14.0)),
        Vec2::new(inner_w, px(32.0)),
    );
    actions::quick_actions(app, ui, quick, project, s);

    let console_rect = Rect::from_min_max(
        Pos2::new(inner_x, quick.bottom() + px(12.0)),
        Pos2::new(inner_x + inner_w, rect.bottom() - pad),
    );
    let cctx = ui.ctx().clone();
    console::show(&mut app.build_ui, ui, console_rect, s, &cctx);
}

fn right_column(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    project: &Project,
    s: f32,
    ctx: &egui::Context,
) {
    let px = |v: f32| theme::px(v, s);
    let run_h = px(350.0);
    let run = Rect::from_min_size(rect.min, Vec2::new(rect.width(), run_h));
    let stages = Rect::from_min_max(
        Pos2::new(rect.left(), run.bottom() + px(20.0)),
        Pos2::new(rect.right(), rect.bottom()),
    );
    run_panel(app, ui, run, project, s, ctx);
    let last = app.history_ui.latest_for(&project.id).cloned();
    stages::show(app, ui, stages, s, last.as_ref());
}

fn run_panel(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    project: &Project,
    s: f32,
    ctx: &egui::Context,
) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    let pad = px(18.0);
    let x = rect.left() + pad;
    let w = rect.width() - pad * 2.0;
    kit::label(
        &ui.painter().clone(),
        Pos2::new(x, rect.top() + pad),
        "RUN",
        s,
    );

    let desc = Rect::from_min_size(Pos2::new(x, rect.top() + px(44.0)), Vec2::new(w, px(40.0)));
    Input::new(&mut app.build_ui.description)
        .hint("Build description")
        .show(ui, desc, s);

    let tog = Rect::from_min_size(
        Pos2::new(x, desc.bottom() + px(14.0)),
        Vec2::new(px(44.0), px(24.0)),
    );
    kit::toggle(ui, tog, &mut app.build_ui.preview_only, s);
    ui.painter().text(
        Pos2::new(tog.right() + px(10.0), tog.center().y),
        Align2::LEFT_CENTER,
        "Preview only",
        FontId::new(px(13.0), theme::family_medium()),
        theme::INK_DIM,
    );

    let mut cx = x;
    let chip_y = tog.bottom() + px(16.0);
    for (i, label) in state::OPTIONS.iter().enumerate() {
        let size = kit::chip_size(ui, label, s);
        if cx + size.x > rect.right() - pad {
            break;
        }
        let r = Rect::from_min_size(Pos2::new(cx, chip_y), size);
        if kit::chip(ui, r, label, app.build_ui.options[i], s).clicked() {
            app.build_ui.options[i] = !app.build_ui.options[i];
        }
        cx += size.x + px(8.0);
    }

    let cli_row = Rect::from_min_size(
        Pos2::new(x, rect.bottom() - pad - px(140.0)),
        Vec2::new(w, px(30.0)),
    );
    actions::cli_actions(app, ui, cli_row, project, s);

    let running = app.build_ui.running;
    let auth_rect = Rect::from_min_size(
        Pos2::new(x, rect.bottom() - pad - px(94.0)),
        Vec2::new(w, px(40.0)),
    );
    let auth_ok = !running && !project.steam_username.trim().is_empty();
    if Button::new("AUTHORIZE ACCOUNT", ButtonKind::Secondary)
        .enabled(auth_ok)
        .show(ui, auth_rect, s)
        .clicked()
        && auth_ok
    {
        if let Err(e) = app.build_ui.authorize(project, ctx) {
            app.build_ui
                .push_log(format!("Could not authorize: {e}"), true);
            app.toasts
                .push(kit::ToastLevel::Error, "Could not authorize");
        } else {
            app.toasts
                .push(kit::ToastLevel::Success, "Authorizing account");
        }
    }

    let btn = Rect::from_min_size(
        Pos2::new(x, rect.bottom() - pad - px(44.0)),
        Vec2::new(w, px(44.0)),
    );
    if !project.pipeline.is_empty() {
        let busy = running || app.pipeline_ui.running;
        if Button::new("RUN PIPELINE", ButtonKind::Primary)
            .icon(theme::icon::FLOW)
            .enabled(!busy)
            .show(ui, btn, s)
            .clicked()
            && !busy
        {
            app.pipeline_ui.select(project);
            app.pipeline_ui.start(ctx, 0);
            app.nav_request = Some(Page::Pipelines);
            app.toasts
                .push(kit::ToastLevel::Success, "Running pipeline");
        }
        return;
    }

    let valid_reason = build_reason(project);
    let t = anim::anim_bool(
        ctx,
        ui.make_persistent_id("run_cancel"),
        running,
        0.18,
        anim::ease_out_cubic,
    );

    if t < 0.999 {
        let mut cu = ui.new_child(UiBuilder::new().max_rect(btn));
        cu.set_opacity(1.0 - t);
        let mut start = Button::new("START BUILD", ButtonKind::Primary)
            .icon(theme::icon::ROCKET)
            .enabled(!running && valid_reason.is_none());
        if let Some(r) = &valid_reason {
            start = start.disabled_reason(r);
        }
        if start.show(&mut cu, btn, s).clicked() && !running && valid_reason.is_none() {
            let branch = project
                .branches
                .get(app.build_ui.branch_sel)
                .map(|b| b.name.clone())
                .unwrap_or_default();
            guard::request(app, project, &branch, ctx);
        }
    }
    if t > 0.001 {
        let mut cu = ui.new_child(UiBuilder::new().max_rect(btn));
        cu.set_opacity(t);
        if Button::new("CANCEL", ButtonKind::Danger)
            .icon(theme::icon::X)
            .enabled(running)
            .show(&mut cu, btn, s)
            .clicked()
            && running
        {
            app.build_ui.cancel();
        }
    }
}

fn build_reason(project: &Project) -> Option<String> {
    if let Some(e) = validate_project(project).first() {
        return Some(format!("{}: {}", e.field, e.message));
    }
    if project.depots.is_empty() {
        return Some("add at least one depot".to_string());
    }
    None
}
