mod branches;
mod depots;
mod editor;
mod preview;
mod scan;
mod state;

pub use state::ProjectsState;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Stroke, Vec2, pos2};

use crate::app::App;
use crate::config;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind};
use crate::ui::{layout::Layout, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let ctx = ui.ctx().clone();
    let s = layout.scale;
    let px = |v: f32| theme::px(v, s);
    app.projects_ui.drain();
    if !app.projects_ui.inited {
        if let Some(p) = app.projects.first().cloned() {
            app.projects_ui.select(&p);
        }
        app.projects_ui.inited = true;
    }

    let content = layout.content;

    if app.projects.is_empty() && app.projects_ui.selected.is_none() {
        empty(app, ui, content, s);
        return;
    }

    let list_w = px(248.0);
    let list = Rect::from_min_max(content.min, pos2(content.left() + list_w, content.bottom()));
    let divider_x = list.right() + px(20.0);
    ui.painter().line_segment(
        [
            Pos2::new(divider_x, content.top() + px(6.0)),
            Pos2::new(divider_x, content.bottom() - px(6.0)),
        ],
        Stroke::new(1.0, theme::LINE),
    );
    let editor_rect = Rect::from_min_max(pos2(divider_x + px(20.0), content.top()), content.max);

    left_panel(app, ui, list, s);
    if editor::show(app, ui, editor_rect, s, &ctx) {
        save_current(app);
    }

    if app.projects_ui.confirm_delete {
        confirm_modal(app, ui, s);
    }
}

fn reload(app: &mut App) {
    app.projects = config::list_projects().unwrap_or_default();
}

fn save_current(app: &mut App) {
    let project = app.projects_ui.project();
    match config::save_project(&project) {
        Ok(()) => {
            reload(app);
            app.projects_ui.mark_saved(project);
            app.toasts.push(kit::ToastLevel::Success, "Project saved");
        }
        Err(e) => {
            eprintln!("save failed: {e}");
            app.toasts.push(kit::ToastLevel::Error, "Save failed");
        }
    }
}

fn empty(app: &mut App, ui: &mut egui::Ui, content: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::hero(&ui.painter().clone(), content, "PROJECTS", s);
    let c = content.center();
    ui.painter().text(
        pos2(c.x, c.y - px(28.0)),
        Align2::CENTER_CENTER,
        "No projects yet",
        FontId::new(px(16.0), theme::family_semibold()),
        theme::INK_DIM,
    );
    ui.painter().text(
        pos2(c.x, c.y + px(2.0)),
        Align2::CENTER_CENTER,
        "Create one to generate VDFs and run builds.",
        FontId::new(px(13.0), theme::family_medium()),
        theme::INK_MUTED,
    );
    let btn = Rect::from_center_size(pos2(c.x, c.y + px(56.0)), Vec2::new(px(188.0), px(44.0)));
    if Button::new("NEW PROJECT", ButtonKind::Primary)
        .icon(theme::icon::PLUS)
        .show(ui, btn, s)
        .clicked()
    {
        app.projects_ui.start_new();
    }
}

enum Action {
    New,
    Duplicate,
    Delete,
    Select(usize),
}

fn left_panel(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    ui.painter().text(
        Pos2::new(rect.left(), rect.top() + px(2.0)),
        Align2::LEFT_TOP,
        "PROJECTS",
        FontId::new(px(13.0), theme::family_label()),
        theme::INK_MUTED,
    );

    let row_y = rect.top() + px(30.0);
    let bw = (rect.width() - px(16.0)) / 3.0;
    let has_sel = app.projects_ui.selected.is_some() && app.projects_ui.on_disk;
    let mut action: Option<Action> = None;

    if Button::new("NEW", ButtonKind::Primary)
        .icon(theme::icon::PLUS)
        .show(
            ui,
            Rect::from_min_size(pos2(rect.left(), row_y), Vec2::new(bw, px(36.0))),
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
                pos2(rect.left() + bw + px(8.0), row_y),
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
                pos2(rect.left() + (bw + px(8.0)) * 2.0, row_y),
                Vec2::new(bw, px(36.0)),
            ),
            s,
        )
        .clicked()
        && has_sel
    {
        action = Some(Action::Delete);
    }

    let mut y = row_y + px(50.0);
    let selected = app.projects_ui.selected.clone();
    for (i, p) in app.projects.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left(), y), Vec2::new(rect.width(), px(46.0)));
        let is_sel = selected.as_deref() == Some(p.id.as_str());
        let resp = ui.interact(r, ui.make_persistent_id(("proj_row", i)), Sense::click());
        let hover = ui.ctx().animate_bool_with_time(
            ui.make_persistent_id(("proj_hover", i)),
            resp.hovered(),
            0.12,
        );
        let fill = if is_sel {
            theme::FIELD
        } else {
            theme::lerp_color(theme::SURFACE, theme::SURFACE_2, hover)
        };
        ui.painter()
            .rect_filled(r, theme::corner(px(theme::RADIUS_FIELD)), fill);
        if is_sel {
            ui.painter().rect_filled(
                Rect::from_min_size(r.min, Vec2::new(px(3.0), r.height())),
                theme::corner(px(2.0)),
                theme::INK,
            );
        }
        let name = if p.name.is_empty() {
            "Untitled"
        } else {
            p.name.as_str()
        };
        ui.painter().text(
            pos2(r.left() + px(16.0), r.center().y - px(8.0)),
            Align2::LEFT_CENTER,
            name,
            FontId::new(px(13.0), theme::family_semibold()),
            if is_sel { theme::INK } else { theme::INK_DIM },
        );
        let sub = if p.app_id.is_empty() {
            "no app id".to_string()
        } else {
            format!("App {}", p.app_id)
        };
        ui.painter().text(
            pos2(r.left() + px(16.0), r.center().y + px(9.0)),
            Align2::LEFT_CENTER,
            sub,
            FontId::new(px(11.0), theme::family_medium()),
            theme::INK_MUTED,
        );
        if resp.clicked() {
            action = Some(Action::Select(i));
        }
        y += px(52.0);
    }

    match action {
        Some(Action::New) => app.projects_ui.start_new(),
        Some(Action::Duplicate) => duplicate_current(app),
        Some(Action::Delete) => app.projects_ui.confirm_delete = true,
        Some(Action::Select(i)) => {
            let p = app.projects[i].clone();
            app.projects_ui.select(&p);
        }
        None => {}
    }
}

fn duplicate_current(app: &mut App) {
    if let Some(id) = app.projects_ui.selected.clone() {
        match config::duplicate_project(&id) {
            Ok(dup) => {
                reload(app);
                app.projects_ui.select(&dup);
            }
            Err(e) => eprintln!("duplicate failed: {e}"),
        }
    }
}

fn confirm_modal(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let name = if app.projects_ui.form.name.is_empty() {
        "this project".to_string()
    } else {
        app.projects_ui.form.name.clone()
    };
    let id = ui.make_persistent_id("delete_modal");
    let mut do_delete = false;
    kit::modal(
        ui,
        id,
        &mut app.projects_ui.confirm_delete,
        Vec2::new(px(390.0), px(188.0)),
        s,
        |ui| {
            let d = ui.max_rect();
            let mut close = false;
            ui.painter().text(
                pos2(d.left() + px(26.0), d.top() + px(28.0)),
                Align2::LEFT_TOP,
                "DELETE PROJECT",
                FontId::new(px(13.0), theme::family_label()),
                theme::INK,
            );
            ui.painter().text(
                pos2(d.left() + px(26.0), d.top() + px(64.0)),
                Align2::LEFT_TOP,
                format!("Delete {name}? This cannot be undone."),
                FontId::new(px(13.0), theme::family_medium()),
                theme::INK_DIM,
            );
            let bw = px(150.0);
            let by = d.bottom() - px(26.0) - px(40.0);
            if Button::new("CANCEL", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(pos2(d.left() + px(26.0), by), Vec2::new(bw, px(40.0))),
                    s,
                )
                .clicked()
            {
                close = true;
            }
            if Button::new("DELETE", ButtonKind::Danger)
                .icon(theme::icon::TRASH)
                .show(
                    ui,
                    Rect::from_min_size(
                        pos2(d.right() - px(26.0) - bw, by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked()
            {
                do_delete = true;
                close = true;
            }
            close
        },
    );
    if do_delete {
        delete_current(app);
    }
}

fn delete_current(app: &mut App) {
    if let Some(id) = app.projects_ui.selected.clone() {
        if let Err(e) = config::delete_project(&id) {
            eprintln!("delete failed: {e}");
        }
        reload(app);
        app.projects_ui.confirm_delete = false;
        if let Some(p) = app.projects.first().cloned() {
            app.projects_ui.select(&p);
        } else {
            app.projects_ui.clear();
        }
        app.toasts.push(kit::ToastLevel::Success, "Project deleted");
    }
}
