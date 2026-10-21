#![allow(dead_code)]

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use super::editor::row;
use super::scan::{ScanState, human_size};
use super::state::PickField;
use crate::app::App;
use crate::model::Depot;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Preset {
    Windows,
    Linux,
    Macos,
}

fn preset_depot(preset: Preset) -> Depot {
    let (name, exclusions): (&str, Vec<&str>) = match preset {
        Preset::Windows => ("Windows Content", vec![".pdb", ".log", ".git*"]),
        Preset::Linux => ("Linux Content", vec![".log", ".git*"]),
        Preset::Macos => ("macOS Content", vec![".DS_Store", ".log", ".git*"]),
    };
    Depot {
        name: name.to_string(),
        depot_path: ".".to_string(),
        recursive: true,
        exclusions: exclusions.into_iter().map(String::from).collect(),
        ..Default::default()
    }
}

enum DepotAct {
    Move(usize, bool),
    Remove(usize),
    Scan(usize),
    AddExcl(usize),
    RemoveExcl(usize, usize),
}

pub(super) fn sections(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) {
    depot_section(app, ui, s, ctx);
    super::branches::branch_section(app, ui, s);
}

pub(super) fn header_row(ui: &mut egui::Ui, title: &str, s: f32) -> Rect {
    let px = |v: f32| theme::px(v, s);
    ui.add_space(px(10.0));
    let hr = row(ui, s, 34.0);
    kit::separator(
        &ui.painter().clone(),
        Pos2::new(hr.left(), hr.top()),
        Pos2::new(hr.right(), hr.top()),
    );
    ui.painter().text(
        Pos2::new(hr.left(), hr.center().y + px(2.0)),
        Align2::LEFT_CENTER,
        title,
        FontId::new(px(13.0), theme::family_semibold()),
        theme::INK,
    );
    hr
}

fn depot_section(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    let hr = header_row(ui, "DEPOTS", s);
    let bw = px(82.0);
    let presets = [
        ("+ WIN", Some(Preset::Windows)),
        ("+ LINUX", Some(Preset::Linux)),
        ("+ MAC", Some(Preset::Macos)),
        ("+ EMPTY", None),
    ];
    for (k, (lbl, preset)) in presets.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(hr.right() - bw * (4 - k) as f32 - px(6.0) * (3 - k) as f32, hr.center().y - px(15.0)),
            Vec2::new(bw, px(30.0)),
        );
        if Button::new(lbl, ButtonKind::Secondary).show(ui, r, s).clicked() {
            app.projects_ui.add_depot(preset.map(preset_depot).unwrap_or_default());
        }
    }

    if app.projects_ui.base.depots.is_empty() {
        let r = row(ui, s, 26.0);
        ui.painter().text(
            r.left_top(),
            Align2::LEFT_TOP,
            "No depots. Add one from a preset above.",
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
    }

    let mut act = None;
    for i in 0..app.projects_ui.base.depots.len() {
        if let Some(a) = depot_card(app, ui, s, ctx, i) {
            act = Some(a);
        }
        ui.add_space(px(10.0));
    }
    match act {
        Some(DepotAct::Move(i, up)) => app.projects_ui.move_depot(i, up),
        Some(DepotAct::Remove(i)) => app.projects_ui.remove_depot(i),
        Some(DepotAct::Scan(i)) => app.projects_ui.scan_depot(ctx, i),
        Some(DepotAct::AddExcl(i)) => {
            if let Some(d) = app.projects_ui.base.depots.get_mut(i) {
                d.exclusions.push(String::new());
            }
        }
        Some(DepotAct::RemoveExcl(i, j)) => {
            if let Some(d) = app.projects_ui.base.depots.get_mut(i)
                && j < d.exclusions.len()
            {
                d.exclusions.remove(j);
            }
        }
        None => {}
    }
}

fn depot_card(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context, i: usize) -> Option<DepotAct> {
    let px = |v: f32| theme::px(v, s);
    let n_excl = app.projects_ui.base.depots[i].exclusions.len();
    let card_h = 196.0 + n_excl as f32 * 34.0;
    let card = row(ui, s, card_h);
    kit::bordered(&ui.painter().clone(), card, theme::SURFACE_2, theme::LINE, px(theme::RADIUS_CARD));
    let pad = px(14.0);
    let x = card.left() + pad;
    let w = card.width() - pad * 2.0;
    let mut y = card.top() + pad;
    let mut act = None;

    let id_w = px(110.0);
    let btn_w = px(46.0);
    let gap = px(6.0);
    let id_err = app.projects_ui.error_for(&format!("depots[{i}].depot_id"));
    let mut tid = Input::new(&mut app.projects_ui.base.depots[i].depot_id).hint("depot id");
    if let Some(e) = &id_err {
        tid = tid.error(true).helper(e);
    }
    tid.show(ui, Rect::from_min_size(Pos2::new(x, y), Vec2::new(id_w, px(40.0))), s);
    let name_w = w - id_w - gap - 3.0 * (btn_w + gap);
    Input::new(&mut app.projects_ui.base.depots[i].name)
        .hint("name")
        .show(ui, Rect::from_min_size(Pos2::new(x + id_w + gap, y), Vec2::new(name_w, px(40.0))), s);
    let mut bx = x + w - 3.0 * btn_w - 2.0 * gap;
    for (lbl, kind, a) in [
        ("UP", ButtonKind::Secondary, DepotAct::Move(i, true)),
        ("DN", ButtonKind::Secondary, DepotAct::Move(i, false)),
        ("X", ButtonKind::Danger, DepotAct::Remove(i)),
    ] {
        if Button::new(lbl, kind)
            .show(ui, Rect::from_min_size(Pos2::new(bx, y), Vec2::new(btn_w, px(40.0))), s)
            .clicked()
        {
            act = Some(a);
        }
        bx += btn_w + gap;
    }
    y += px(50.0);

    let scan_w = px(88.0);
    let browse_w = px(96.0);
    let path_w = w - browse_w - scan_w - px(12.0) - px(150.0);
    let pr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(path_w, px(40.0)));
    kit::bordered(&ui.painter().clone(), pr, theme::FIELD, theme::LINE, px(theme::RADIUS_FIELD));
    let lp = app.projects_ui.base.depots[i].local_path.to_string_lossy().to_string();
    let (txt, col) = if lp.is_empty() {
        ("local path".to_string(), theme::INK_FAINT)
    } else {
        (lp, theme::INK_DIM)
    };
    ui.painter().text(
        Pos2::new(pr.left() + px(14.0), pr.center().y),
        Align2::LEFT_CENTER,
        txt,
        FontId::new(px(12.0), theme::family_medium()),
        col,
    );
    if Button::new("BROWSE", ButtonKind::Secondary)
        .icon(theme::icon::FOLDER)
        .show(ui, Rect::from_min_size(Pos2::new(x + path_w + px(6.0), y), Vec2::new(browse_w, px(40.0))), s)
        .clicked()
    {
        app.projects_ui.spawn_pick(ctx, PickField::DepotLocal, i);
    }
    let running = matches!(app.projects_ui.scans.get(i), Some(ScanState::Running));
    if Button::new("SCAN", ButtonKind::Secondary)
        .loading(running)
        .show(ui, Rect::from_min_size(Pos2::new(x + path_w + browse_w + px(12.0), y), Vec2::new(scan_w, px(40.0))), s)
        .clicked()
        && !running
    {
        act = Some(DepotAct::Scan(i));
    }
    if let Some(ScanState::Done { files, bytes }) = app.projects_ui.scans.get(i).copied() {
        ui.painter().text(
            Pos2::new(x + w, y + px(50.0)),
            Align2::RIGHT_CENTER,
            format!("{files} files  {}", human_size(bytes)),
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
    }
    y += px(50.0);

    Input::new(&mut app.projects_ui.base.depots[i].depot_path)
        .hint(".")
        .show(ui, Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(120.0), px(36.0))), s);
    kit::checkbox(
        ui,
        Rect::from_min_size(Pos2::new(x + px(140.0), y), Vec2::new(px(160.0), px(36.0))),
        &mut app.projects_ui.base.depots[i].recursive,
        "RECURSIVE",
        s,
    );
    y += px(44.0);

    ui.painter().text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        "EXCLUSIONS",
        FontId::new(px(11.0), theme::family_label()),
        theme::INK_MUTED,
    );
    y += px(18.0);
    for j in 0..app.projects_ui.base.depots[i].exclusions.len() {
        Input::new(&mut app.projects_ui.base.depots[i].exclusions[j])
            .hint("*.pdb")
            .show(ui, Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(220.0), px(28.0))), s);
        if Button::new("X", ButtonKind::Danger)
            .show(ui, Rect::from_min_size(Pos2::new(x + px(228.0), y), Vec2::new(px(38.0), px(28.0))), s)
            .clicked()
        {
            act = Some(DepotAct::RemoveExcl(i, j));
        }
        y += px(34.0);
    }
    if Button::new("+ PATTERN", ButtonKind::Secondary)
        .show(ui, Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(128.0), px(30.0))), s)
        .clicked()
    {
        act = Some(DepotAct::AddExcl(i));
    }
    act
}
