use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use super::editor::{label, row};
use crate::app::App;
use crate::features::checklist;
use crate::model::ChecklistItem;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::theme;

pub(super) fn sections(app: &mut App, ui: &mut egui::Ui, s: f32) {
    default_account(app, ui, s);
    checklist_section(app, ui, s);
}

fn default_account(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    label(ui, "Default account", s);
    let accounts = crate::config::load_settings()
        .map(|c| c.accounts)
        .unwrap_or_default();
    let r = row(ui, s, 44.0);
    if accounts.is_empty() {
        ui.painter().text(
            Pos2::new(r.left(), r.top() + px(12.0)),
            Align2::LEFT_TOP,
            "Add accounts in Settings > Accounts, then choose one here.",
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
        ui.add_space(px(8.0));
        return;
    }
    let current = app.projects_ui.base.default_account.clone();
    let mut sel = accounts
        .iter()
        .position(|a| a.eq_ignore_ascii_case(&current))
        .unwrap_or(0);
    let refs: Vec<&str> = accounts.iter().map(|x| x.as_str()).collect();
    let dd = Rect::from_min_size(r.min, Vec2::new(px(260.0), px(40.0)));
    kit::dropdown(ui, dd, &refs, &mut sel, s);
    if let Some(name) = accounts.get(sel)
        && !name.eq_ignore_ascii_case(&current)
    {
        app.projects_ui.base.default_account = name.clone();
        app.projects_ui.form.username = name.clone();
    }
    ui.add_space(px(10.0));
}

fn checklist_section(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    label(ui, "Release checklist", s);

    let header = row(ui, s, 40.0);
    let add = Rect::from_min_size(header.min, Vec2::new(px(120.0), px(34.0)));
    if Button::new("Add item", ButtonKind::Secondary)
        .icon(theme::icon::PLUS)
        .show(ui, add, s)
        .clicked()
    {
        app.projects_ui
            .base
            .checklist
            .push(ChecklistItem::default());
    }
    let tmpl = Rect::from_min_size(
        Pos2::new(add.right() + px(10.0), header.top()),
        Vec2::new(px(140.0), px(34.0)),
    );
    if Button::new("Load template", ButtonKind::Secondary)
        .show(ui, tmpl, s)
        .clicked()
    {
        app.projects_ui.base.checklist = checklist::template();
    }
    let done = app
        .projects_ui
        .base
        .checklist
        .iter()
        .filter(|i| i.done)
        .count();
    let total = app.projects_ui.base.checklist.len();
    ui.painter().text(
        Pos2::new(header.right() - px(4.0), header.center().y),
        Align2::RIGHT_CENTER,
        format!("{done}/{total} done"),
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK_MUTED,
    );
    ui.add_space(px(6.0));

    let tr = row(ui, s, 30.0);
    let tog = Rect::from_min_size(tr.min, Vec2::new(px(44.0), px(24.0)));
    let mut cfg = crate::config::load_settings().unwrap_or_default();
    let mut reset_on = cfg.checklist_reset_on_live;
    if kit::toggle(ui, tog, &mut reset_on, s).changed() {
        cfg.checklist_reset_on_live = reset_on;
        if let Err(e) = crate::config::save_settings(&cfg) {
            eprintln!("could not save settings: {e}");
        }
    }
    ui.painter().text(
        Pos2::new(tog.right() + px(12.0), tog.center().y),
        Align2::LEFT_CENTER,
        "Reset the checklist automatically after each Live build",
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK_DIM,
    );
    ui.add_space(px(8.0));

    let mut remove = None;
    let len = app.projects_ui.base.checklist.len();
    for i in 0..len {
        let r = row(ui, s, 46.0);
        let cb = Rect::from_min_size(
            Pos2::new(r.left(), r.top() + px(8.0)),
            Vec2::splat(px(24.0)),
        );
        let mut done = app.projects_ui.base.checklist[i].done;
        if kit::checkbox(ui, cb, &mut done, "", s).changed() {
            app.projects_ui.base.checklist[i].done = done;
        }
        let xz = Rect::from_min_size(
            Pos2::new(r.right() - px(34.0), r.top() + px(5.0)),
            Vec2::splat(px(32.0)),
        );
        let field = Rect::from_min_size(
            Pos2::new(cb.right() + px(10.0), r.top() + px(4.0)),
            Vec2::new(r.width() - px(80.0), px(36.0)),
        );
        Input::new(&mut app.projects_ui.base.checklist[i].text)
            .hint("Checklist item")
            .show(ui, field, s);
        if kit::icon_button(ui, xz, theme::icon::TRASH, s).clicked() {
            remove = Some(i);
        }
        ui.add_space(px(4.0));
    }
    if let Some(i) = remove {
        app.projects_ui.base.checklist.remove(i);
    }
    ui.add_space(px(10.0));
}
