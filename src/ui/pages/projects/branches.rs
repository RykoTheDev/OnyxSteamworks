#![allow(dead_code)]

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use super::depots::header_row;
use super::editor::{label, row};
use crate::app::App;
use crate::model::BranchLevel;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

pub(super) fn branch_section(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let hr = header_row(ui, "BRANCHES", s);
    if Button::new("+ BRANCH", ButtonKind::Secondary)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(hr.right() - px(108.0), hr.center().y - px(15.0)),
                Vec2::new(px(108.0), px(30.0)),
            ),
            s,
        )
        .clicked()
    {
        app.projects_ui.add_branch();
    }

    let mut remove = None;
    for i in 0..app.projects_ui.base.branches.len() {
        let r = row(ui, s, 48.0);
        Input::new(&mut app.projects_ui.base.branches[i].name)
            .hint("branch name")
            .show(
                ui,
                Rect::from_min_size(r.min, Vec2::new(px(200.0), px(40.0))),
                s,
            );
        let mut lvl = match app.projects_ui.base.branches[i].level {
            BranchLevel::Dev => 0,
            BranchLevel::Beta => 1,
            BranchLevel::Live => 2,
        };
        kit::dropdown(
            ui,
            Rect::from_min_size(
                Pos2::new(r.left() + px(210.0), r.top()),
                Vec2::new(px(140.0), px(40.0)),
            ),
            &["DEV", "BETA", "LIVE"],
            &mut lvl,
            s,
        );
        app.projects_ui.base.branches[i].level =
            [BranchLevel::Dev, BranchLevel::Beta, BranchLevel::Live][lvl];
        if Button::new("X", ButtonKind::Danger)
            .show(
                ui,
                Rect::from_min_size(
                    Pos2::new(r.left() + px(360.0), r.top()),
                    Vec2::new(px(44.0), px(40.0)),
                ),
                s,
            )
            .clicked()
        {
            remove = Some(i);
        }
        ui.add_space(px(6.0));
    }
    if let Some(i) = remove {
        app.projects_ui.remove_branch(i);
    }

    ui.add_space(px(6.0));
    label(ui, "SET LIVE BRANCH", s);
    let names: Vec<String> = app
        .projects_ui
        .base
        .branches
        .iter()
        .map(|b| b.name.clone())
        .collect();
    if names.is_empty() {
        let r = row(ui, s, 24.0);
        ui.painter().text(
            r.left_top(),
            Align2::LEFT_TOP,
            "Add a branch first",
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
        return;
    }
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let mut sel = refs
        .iter()
        .position(|n| *n == app.projects_ui.base.set_live_branch)
        .unwrap_or(0);
    let r = row(ui, s, 44.0);
    kit::dropdown(
        ui,
        Rect::from_min_size(r.min, Vec2::new(px(240.0), px(40.0))),
        &refs,
        &mut sel,
        s,
    );
    app.projects_ui.base.set_live_branch = refs.get(sel).map(|x| x.to_string()).unwrap_or_default();
    if app
        .projects_ui
        .base
        .set_live_branch
        .eq_ignore_ascii_case("default")
    {
        let w = row(ui, s, 24.0);
        ui.painter().text(
            w.left_top(),
            Align2::LEFT_TOP,
            "SteamCMD cannot set the default branch live",
            FontId::new(px(12.0), theme::family_medium()),
            theme::DANGER,
        );
    }
}
