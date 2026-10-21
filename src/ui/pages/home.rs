mod cards;
mod state;

pub use state::HomeState;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, UiBuilder, Vec2};

use crate::app::{App, Page};
use crate::model::Project;
use crate::ui::kit::{self, Input};
use crate::ui::layout::Layout;
use crate::ui::{anim, nav, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let s = layout.scale;
    let px = |v: f32| theme::px(v, s);
    let content = layout.content;
    let ctx = ui.ctx().clone();

    kit::hero(&ui.painter().clone(), content, "STEAMPIPE", s);
    let body_top = content.top() + px(104.0);

    if app.projects.is_empty() {
        let body = Rect::from_min_max(Pos2::new(content.left(), body_top), content.max);
        kit::empty_state(
            ui,
            body,
            theme::icon::PACKAGE,
            "No project yet",
            "Create a project to generate VDFs and run builds from here.",
            s,
        );
        return;
    }

    if app.build_ui.project_sel >= app.projects.len() {
        app.build_ui.project_sel = 0;
    }
    let project = app.projects[app.build_ui.project_sel].clone();

    let right_w = px(324.0);
    let gap = px(22.0);
    let left = Rect::from_min_max(
        Pos2::new(content.left(), body_top),
        Pos2::new(content.right() - right_w - gap, content.bottom()),
    );
    let right = Rect::from_min_max(Pos2::new(content.right() - right_w, body_top), content.max);

    let (op, _) = anim::stagger_in(&ctx, ui.make_persistent_id(("home", app.home_gen)), 0, 0.0);
    let mut cu = ui.new_child(UiBuilder::new().max_rect(content));
    cu.set_opacity(op);
    {
        let ui = &mut cu;
        left_main(app, ui, left, &project, s, &ctx);
        let choose = Rect::from_min_size(right.min, Vec2::new(right_w, px(236.0)));
        let notes = Rect::from_min_max(
            Pos2::new(right.left(), choose.bottom() + px(18.0)),
            right.max,
        );
        cards::choose_branch(app, ui, choose, &project, s);
        cards::patch_notes(app, ui, notes, &project, s);
    }

    crate::ui::pages::build::preflight::show(app, ui, s, &ctx);
    crate::ui::pages::build::review::show(app, ui, s, &ctx);
}

fn left_main(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    project: &Project,
    s: f32,
    ctx: &egui::Context,
) {
    let px = |v: f32| theme::px(v, s);
    let x = rect.left();
    let w = rect.width();

    let name = if project.name.is_empty() {
        "Untitled project".to_string()
    } else {
        project.name.clone()
    };
    ui.painter().text(
        Pos2::new(x, rect.top()),
        Align2::LEFT_TOP,
        &name,
        FontId::new(px(20.0), theme::family_semibold()),
        theme::INK,
    );
    let sub = if project.app_id.is_empty() {
        "No app id set".to_string()
    } else {
        format!("App {}", project.app_id)
    };
    ui.painter().text(
        Pos2::new(x, rect.top() + px(28.0)),
        Align2::LEFT_TOP,
        sub,
        FontId::new(px(13.0), theme::family_medium()),
        theme::INK_MUTED,
    );

    let art = Rect::from_min_size(Pos2::new(x, rect.top() + px(52.0)), Vec2::new(w, px(130.0)));
    let key_art = project.key_art_path.to_string_lossy();
    let uri = (!key_art.is_empty()).then(|| kit::file_uri(&key_art));
    let letter = name.chars().next().unwrap_or('S');
    kit::image_or_placeholder(ui, art, uri.as_deref(), letter, px(theme::RADIUS_CARD), s);

    let pb = Rect::from_min_size(
        Pos2::new(x, art.bottom() + px(18.0)),
        Vec2::new(w, px(50.0)),
    );
    prompt_bar(app, ui, pb, project, s, ctx);

    kit::label(
        &ui.painter().clone(),
        Pos2::new(x, pb.bottom() + px(20.0)),
        "RECENT BUILDS",
        s,
    );
    let list = Rect::from_min_max(
        Pos2::new(x, pb.bottom() + px(46.0)),
        Pos2::new(rect.right(), rect.bottom()),
    );

    let records = app.history_ui.for_project(&project.id);
    let total = records.len();
    let shown: Vec<_> = records.into_iter().take(3).collect();
    app.home_ui.ensure(&shown, ctx);
    if shown.is_empty() {
        ui.painter().text(
            Pos2::new(x, list.top() + px(6.0)),
            Align2::LEFT_TOP,
            "No builds yet. Run one and it will show up here.",
            FontId::new(px(13.0), theme::family_medium()),
            theme::INK_MUTED,
        );
    } else {
        let owned: Vec<crate::model::BuildRecord> = shown.into_iter().cloned().collect();
        cards::stacked(app, ui, list, &owned, total, s);
    }
}

fn prompt_bar(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    project: &Project,
    s: f32,
    ctx: &egui::Context,
) {
    let px = |v: f32| theme::px(v, s);
    let h = rect.height();
    let sparkle = Rect::from_min_size(rect.min, Vec2::splat(h));
    let notch = Rect::from_min_size(Pos2::new(rect.right() - h, rect.top()), Vec2::splat(h));
    let field = Rect::from_min_max(
        Pos2::new(sparkle.right() + px(12.0), rect.top() + px(5.0)),
        Pos2::new(notch.left() - px(12.0), rect.bottom() - px(5.0)),
    );

    Input::new(&mut app.build_ui.description)
        .hint("Describe this build...")
        .show(ui, field, s);

    if circle_button(ui, sparkle, theme::icon::SPARKLE, false, s).clicked() {
        app.patchnotes_ui.project_sel = app.build_ui.project_sel;
        crate::ui::pages::history::start_patch_notes(app, false, ctx);
        open_history(app, 2);
    }

    let busy = app.build_ui.running || app.pipeline_ui.running;
    if circle_button(ui, notch, theme::icon::PLAY, !busy, s).clicked() && !busy {
        run_from_home(app, project, ctx);
    }
}

fn run_from_home(app: &mut App, project: &Project, ctx: &egui::Context) {
    if app.history_ui.latest_for(&project.id).is_none() {
        app.build_ui.preview_only = true;
    }
    let branch = project
        .branches
        .get(app.build_ui.branch_sel)
        .map(|b| b.name.clone())
        .unwrap_or_default();
    app.build_ui.preflight.request(project, &branch, ctx);
}

pub(super) fn circle_button(
    ui: &mut egui::Ui,
    rect: Rect,
    glyph: char,
    primary: bool,
    s: f32,
) -> egui::Response {
    let id = ui.make_persistent_id(("circlebtn", glyph, rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    let ctx = ui.ctx().clone();
    let hover = crate::ui::anim::anim_bool(
        &ctx,
        id.with("h"),
        resp.hovered(),
        0.12,
        crate::ui::anim::ease_out_cubic,
    );
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.5;
    let (fill, ink) = if primary {
        (
            theme::lerp_color(theme::PRIMARY, theme::INK, hover * 0.4),
            theme::PRIMARY_INK,
        )
    } else {
        (
            theme::lerp_color(theme::FIELD, theme::FIELD_HI, hover),
            theme::lerp_color(theme::INK_DIM, theme::INK, hover),
        )
    };
    let painter = ui.painter();
    painter.circle_filled(center, radius, fill);
    painter.text(
        center,
        Align2::CENTER_CENTER,
        glyph.to_string(),
        theme::icon_font(theme::px(16.0, s), primary),
        ink,
    );
    resp
}

pub(super) fn open_history(app: &mut App, sub_tab: usize) {
    app.sub_tabs[nav::index(Page::History)] = sub_tab;
    app.nav_request = Some(Page::History);
}
