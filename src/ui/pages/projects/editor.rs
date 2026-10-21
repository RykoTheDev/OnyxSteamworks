use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, UiBuilder, Vec2};

use super::state::PickField;
use crate::app::App;
use crate::features::template;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32, ctx: &egui::Context) -> bool {
    let px = |v: f32| theme::px(v, s);

    kit::hero(&ui.painter().clone(), rect, "PROJECTS", s);
    if app.projects_ui.dirty() {
        ui.painter().circle_filled(
            Pos2::new(rect.right() - px(14.0), rect.top() + px(14.0)),
            px(4.0),
            theme::WARNING,
        );
    }

    let tab_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + px(92.0)),
        Vec2::new(px(240.0), px(32.0)),
    );
    kit::tabs(
        ui,
        tab_rect,
        &["EDITOR", "VDF PREVIEW"],
        &mut app.projects_ui.editor_tab,
        s,
    );

    let save_h = px(66.0);
    let content = Rect::from_min_max(
        Pos2::new(rect.left(), tab_rect.bottom() + px(18.0)),
        Pos2::new(rect.right(), rect.bottom() - save_h),
    );

    if app.projects_ui.editor_tab == 1 {
        super::preview::show(app, ui, content, s, ctx);
    } else {
        let mut changed = false;
        let mut cui = ui.new_child(UiBuilder::new().max_rect(content));
        egui::ScrollArea::vertical()
            .id_salt("editor_scroll")
            .auto_shrink([false, false])
            .show(&mut cui, |ui| {
                ui.set_width(content.width() - px(16.0));
                changed |= basic_fields(app, ui, s, ctx);
                super::depots::sections(app, ui, s, ctx);
                super::checklist::sections(app, ui, s);
                ui.add_space(px(8.0));
            });
        if changed {
            app.projects_ui.revalidate();
        }
    }

    let valid = app.projects_ui.valid();
    let dirty = app.projects_ui.dirty();
    let reason = app.projects_ui.first_error().unwrap_or_default();
    let save_rect = Rect::from_min_size(
        Pos2::new(rect.right() - px(150.0), rect.bottom() - px(44.0)),
        Vec2::new(px(150.0), px(44.0)),
    );
    let mut btn = Button::new("SAVE PROJECT", ButtonKind::Primary)
        .icon(theme::icon::CHECK)
        .enabled(valid && dirty);
    if !valid {
        btn = btn.disabled_reason(&reason);
    }
    btn.show(ui, save_rect, s).clicked() && valid && dirty
}

fn basic_fields(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) -> bool {
    let px = |v: f32| theme::px(v, s);
    let mut changed = false;

    let r = row(ui, s, 64.0);
    let gap = px(18.0);
    let left_w = (r.width() - gap) * 0.42;
    let right_w = r.width() - gap - left_w;
    let left = Rect::from_min_size(r.min, Vec2::new(left_w, r.height()));
    let right = Rect::from_min_size(
        Pos2::new(r.left() + left_w + gap, r.top()),
        Vec2::new(right_w, r.height()),
    );
    changed |= labelled_text(app, ui, left, "APP ID", None, false, s);
    changed |= labelled_text(app, ui, right, "DISPLAY NAME", Some("(Optional)"), false, s);
    ui.add_space(px(22.0));

    changed |= labelled_path(app, ui, "CONTENT ROOT", PickField::ContentRoot, s, ctx);
    changed |= labelled_path(app, ui, "BUILD OUTPUT", PickField::BuildOutput, s, ctx);
    changed |= labelled_path(app, ui, "STEAMCMD PATH", PickField::SteamCmd, s, ctx);
    changed |= labelled_path(app, ui, "KEY ART", PickField::KeyArt, s, ctx);

    let r = row(ui, s, 64.0);
    let uw = (r.width() - gap) * 0.5;
    let uleft = Rect::from_min_size(r.min, Vec2::new(uw, r.height()));
    let uright = Rect::from_min_size(
        Pos2::new(r.left() + uw + gap, r.top()),
        Vec2::new(uw, r.height()),
    );
    changed |= labelled_text(app, ui, uleft, "STEAM USERNAME", None, false, s);
    password_field(app, ui, uright, s);
    let hint = row(ui, s, 22.0);
    ui.painter().text(
        hint.left_top(),
        Align2::LEFT_TOP,
        "Password is stored in your OS keyring and used to sign in. Leave blank to be prompted; Steam Guard is only asked if the cached sign-in expires.",
        FontId::new(px(11.0), theme::family_medium()),
        theme::INK_MUTED,
    );
    ui.add_space(px(8.0));

    template_fields(app, ui, s);
    changed
}

fn template_fields(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    label(ui, "DESCRIPTION TEMPLATE", s);
    let r = row(ui, s, 40.0);
    Input::new(&mut app.projects_ui.base.desc_template)
        .hint("v{version} {branch} {date}.{time} #{build_number} {git_commit}")
        .show(
            ui,
            Rect::from_min_size(r.min, Vec2::new(r.width(), px(40.0))),
            s,
        );
    ui.add_space(px(10.0));

    let branch = app.projects_ui.base.set_live_branch.clone();
    let project = app.projects_ui.project();
    let vars = template::resolve(
        &project,
        if branch.is_empty() {
            "default"
        } else {
            &branch
        },
    );
    let resolved = template::expand(&project.desc_template, &vars);
    let preview = if resolved.trim().is_empty() {
        "(type a template above)".to_string()
    } else {
        resolved
    };
    label(ui, "RESOLVED PREVIEW", s);
    let pr = row(ui, s, 36.0);
    let block = Rect::from_min_size(pr.min, Vec2::new(pr.width(), px(32.0)));
    kit::bordered(
        &ui.painter().clone(),
        block,
        theme::FIELD,
        theme::LINE,
        px(theme::RADIUS_FIELD),
    );
    ui.painter().text(
        Pos2::new(block.left() + px(12.0), block.center().y),
        Align2::LEFT_CENTER,
        preview,
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_DIM,
    );
    ui.add_space(px(10.0));

    label(ui, "VERSION FILE", s);
    let vr = row(ui, s, 40.0);
    Input::new(&mut app.projects_ui.base.version_file)
        .hint("version.txt or version.json in the content root (optional)")
        .show(
            ui,
            Rect::from_min_size(vr.min, Vec2::new(vr.width(), px(40.0))),
            s,
        );
    ui.add_space(px(14.0));
}

pub(super) fn row(ui: &mut egui::Ui, s: f32, h: f32) -> Rect {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), theme::px(h, s)),
        Sense::hover(),
    );
    rect
}

pub(super) fn label(ui: &mut egui::Ui, text: &str, s: f32) {
    let r = row(ui, s, 20.0);
    kit::label(&ui.painter().clone(), r.left_top(), text, s);
}

fn key_for(label: &str) -> &'static str {
    match label {
        "DISPLAY NAME" => "name",
        "APP ID" => "app_id",
        "CONTENT ROOT" => "content_root",
        "BUILD OUTPUT" => "build_output_dir",
        "STEAMCMD PATH" => "steamcmd_path",
        _ => "",
    }
}

fn value_mut<'a>(app: &'a mut App, label: &str) -> &'a mut String {
    match label {
        "DISPLAY NAME" => &mut app.projects_ui.form.name,
        "APP ID" => &mut app.projects_ui.form.app_id,
        "CONTENT ROOT" => &mut app.projects_ui.form.content_root,
        "BUILD OUTPUT" => &mut app.projects_ui.form.build_output,
        "STEAMCMD PATH" => &mut app.projects_ui.form.steamcmd,
        "KEY ART" => &mut app.projects_ui.form.key_art,
        "STEAM USERNAME" => &mut app.projects_ui.form.username,
        _ => unreachable!("unknown editor field: {label}"),
    }
}

fn password_field(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::label(&ui.painter().clone(), rect.left_top(), "STEAM PASSWORD", s);
    let field = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + px(24.0)),
        Vec2::new(rect.width(), px(40.0)),
    );
    let resp = Input::new(&mut app.projects_ui.form.password)
        .hint("Optional — saved to keyring")
        .mask(true)
        .show(ui, field, s);
    if resp.lost_focus() {
        app.projects_ui.store_password();
    }
}

fn labelled_text(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    lbl: &str,
    suffix: Option<&str>,
    mask: bool,
    s: f32,
) -> bool {
    let px = |v: f32| theme::px(v, s);
    let painter = ui.painter().clone();
    match suffix {
        Some(suf) => kit::label_optional(&painter, rect.left_top(), lbl, suf, s),
        None => kit::label(&painter, rect.left_top(), lbl, s),
    }
    let field = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + px(24.0)),
        Vec2::new(rect.width(), px(40.0)),
    );
    let err = app.projects_ui.error_for(key_for(lbl));
    let value = value_mut(app, lbl);
    let mut ti = Input::new(value).mask(mask);
    if let Some(e) = &err {
        ti = ti.error(true).helper(e);
    }
    ti.show(ui, field, s).changed()
}

fn labelled_path(
    app: &mut App,
    ui: &mut egui::Ui,
    lbl: &str,
    field: PickField,
    s: f32,
    ctx: &egui::Context,
) -> bool {
    let px = |v: f32| theme::px(v, s);
    label(ui, lbl, s);
    let r = row(ui, s, 48.0);
    let browse_w = px(104.0);
    let gap = px(8.0);
    let input = Rect::from_min_size(r.min, Vec2::new(r.width() - browse_w - gap, px(40.0)));
    let err = app.projects_ui.error_for(key_for(lbl));
    let value = value_mut(app, lbl);
    let mut ti = Input::new(value);
    if let Some(e) = &err {
        ti = ti.error(true).helper(e);
    }
    let changed = ti.show(ui, input, s).changed();
    let browse = Rect::from_min_size(
        Pos2::new(input.right() + gap, input.top()),
        Vec2::new(browse_w, px(40.0)),
    );
    if Button::new("BROWSE", ButtonKind::Secondary)
        .icon(theme::icon::FOLDER)
        .show(ui, browse, s)
        .clicked()
    {
        app.projects_ui.spawn_pick(ctx, field, 0);
    }
    ui.add_space(if err.is_some() { px(34.0) } else { px(18.0) });
    changed
}
