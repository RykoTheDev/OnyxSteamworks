#![allow(dead_code)]

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use super::editor::row;
use super::scan::{ScanState, human_size};
use super::state::PickField;
use crate::app::App;
use crate::model::Depot;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;
use crate::ui::{anim, kit};

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

fn collapsible_title(
    ui: &mut egui::Ui,
    hr: Rect,
    right: f32,
    title: &str,
    count: usize,
    open: bool,
    s: f32,
) -> bool {
    let px = |v: f32| theme::px(v, s);
    let hit = Rect::from_min_max(
        Pos2::new(hr.left() - px(6.0), hr.top() + px(4.0)),
        Pos2::new(right - px(12.0), hr.bottom()),
    );
    let id = ui.make_persistent_id(("collapsible", title));
    let resp = ui
        .interact(hit, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = anim::anim_bool(
        ui.ctx(),
        id.with("h"),
        resp.hovered(),
        0.12,
        anim::ease_out_cubic,
    );
    let turn = anim::anim_bool(ui.ctx(), id.with("o"), open, 0.16, anim::ease_out_cubic);
    let painter = ui.painter();
    let cy = hr.center().y + px(2.0);
    let galley = painter.layout_no_wrap(
        kit::rail::CARET_RIGHT.to_string(),
        theme::icon_font(px(11.0), true),
        theme::lerp_color(theme::INK_MUTED, theme::INK, hover),
    );
    let angle = turn * std::f32::consts::FRAC_PI_2;
    let half = galley.size() / 2.0;
    let center = Pos2::new(hr.left() + px(6.0), cy);
    let pos = center - egui::emath::Rot2::from_angle(angle) * half;
    painter.add(egui::epaint::TextShape::new(pos, galley, theme::INK).with_angle(angle));
    let title_r = painter.text(
        Pos2::new(hr.left() + px(18.0), cy),
        Align2::LEFT_CENTER,
        title,
        FontId::new(px(13.0), theme::family_semibold()),
        theme::INK,
    );
    let font = FontId::new(px(11.0), theme::family_medium());
    let label = count.to_string();
    let w = painter
        .layout_no_wrap(label.clone(), font.clone(), theme::INK_MUTED)
        .size()
        .x;
    let badge = Rect::from_min_size(
        Pos2::new(title_r.right() + px(8.0), cy - px(9.0)),
        Vec2::new(w + px(12.0), px(18.0)),
    );
    painter.rect_filled(badge, theme::corner(badge.height() / 2.0), theme::FIELD_HI);
    painter.text(
        badge.center(),
        Align2::CENTER_CENTER,
        label,
        font,
        theme::INK_MUTED,
    );
    resp.clicked()
}

fn depot_section(app: &mut App, ui: &mut egui::Ui, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    let hr = header_row(ui, "", s);
    let bw = px(86.0);
    let presets = [
        ("+ Windows", Some(Preset::Windows)),
        ("+ Linux", Some(Preset::Linux)),
        ("+ macOS", Some(Preset::Macos)),
        ("+ Empty", None),
    ];
    let buttons_left = hr.right() - bw * 4.0 - px(6.0) * 3.0;
    let open_id = ui.make_persistent_id("depots_open");
    let mut open = ctx.data(|d| d.get_temp::<bool>(open_id).unwrap_or(true));
    let count = app.projects_ui.base.depots.len();
    if collapsible_title(ui, hr, buttons_left, "Depots", count, open, s) {
        open = !open;
    }
    for (k, (lbl, preset)) in presets.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(
                hr.right() - bw * (4 - k) as f32 - px(6.0) * (3 - k) as f32,
                hr.center().y - px(15.0),
            ),
            Vec2::new(bw, px(30.0)),
        );
        if Button::new(lbl, ButtonKind::Secondary)
            .show(ui, r, s)
            .clicked()
        {
            app.projects_ui
                .add_depot(preset.map(preset_depot).unwrap_or_default());
            open = true;
        }
    }
    ctx.data_mut(|d| d.insert_temp(open_id, open));
    if !open {
        return;
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

fn depot_card(
    app: &mut App,
    ui: &mut egui::Ui,
    s: f32,
    ctx: &egui::Context,
    i: usize,
) -> Option<DepotAct> {
    let px = |v: f32| theme::px(v, s);
    let n_depots = app.projects_ui.base.depots.len();
    let n_excl = app.projects_ui.base.depots[i].exclusions.len();
    let id_err = app.projects_ui.error_for(&format!("depots[{i}].depot_id"));

    const PAD: f32 = 16.0;
    const ROW_H: f32 = 40.0;
    const ROW_GAP: f32 = 10.0;
    const OPT_H: f32 = 36.0;
    const LABEL_H: f32 = 18.0;
    const EXCL_H: f32 = 28.0;
    const EXCL_STEP: f32 = 34.0;
    const ADD_H: f32 = 30.0;
    let err_h = if id_err.is_some() { 18.0 } else { 0.0 };
    let card_h = PAD
        + ROW_H
        + err_h
        + ROW_GAP
        + ROW_H
        + ROW_GAP
        + OPT_H
        + ROW_GAP
        + 4.0
        + LABEL_H
        + n_excl as f32 * EXCL_STEP
        + ADD_H
        + PAD;
    let card = row(ui, s, card_h);
    kit::bordered(
        &ui.painter().clone(),
        card,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_CARD),
    );
    let pad = px(PAD);
    let x = card.left() + pad;
    let w = card.width() - pad * 2.0;
    let mut y = card.top() + pad;
    let mut act = None;

    let id_w = px(110.0);
    let btn_w = px(46.0);
    let gap = px(6.0);
    let mut tid = Input::new(&mut app.projects_ui.base.depots[i].depot_id).hint("depot id");
    if let Some(e) = &id_err {
        tid = tid.error(true).helper(e);
    }
    tid.show(
        ui,
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(id_w, px(ROW_H))),
        s,
    );
    let name_w = w - id_w - gap - 3.0 * (btn_w + gap);
    Input::new(&mut app.projects_ui.base.depots[i].name)
        .hint("name")
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x + id_w + gap, y), Vec2::new(name_w, px(ROW_H))),
            s,
        );
    let mut bx = x + w - 3.0 * btn_w - 2.0 * gap;
    for (lbl, kind, enabled, a) in [
        ("UP", ButtonKind::Secondary, i > 0, DepotAct::Move(i, true)),
        (
            "DN",
            ButtonKind::Secondary,
            i + 1 < n_depots,
            DepotAct::Move(i, false),
        ),
        ("X", ButtonKind::Danger, true, DepotAct::Remove(i)),
    ] {
        if Button::new(lbl, kind)
            .enabled(enabled)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(bx, y), Vec2::new(btn_w, px(ROW_H))),
                s,
            )
            .clicked()
        {
            act = Some(a);
        }
        bx += btn_w + gap;
    }
    y += px(ROW_H + err_h + ROW_GAP);

    let scan_w = px(88.0);
    let browse_w = px(110.0);
    let path_w = w - browse_w - scan_w - 2.0 * gap;
    let pr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(path_w, px(ROW_H)));
    kit::bordered(
        &ui.painter().clone(),
        pr,
        theme::FIELD,
        theme::LINE,
        px(theme::RADIUS_FIELD),
    );
    let lp = app.projects_ui.base.depots[i]
        .local_path
        .to_string_lossy()
        .to_string();
    let (txt, col) = if lp.is_empty() {
        ("local path".to_string(), theme::INK_FAINT)
    } else {
        (lp.clone(), theme::INK_DIM)
    };
    let text_clip = pr.shrink2(Vec2::new(px(14.0), 0.0));
    let galley =
        ui.painter()
            .layout_no_wrap(txt, FontId::new(px(12.0), theme::family_medium()), col);
    let overflow = galley.size().x > text_clip.width();
    let tx = if overflow {
        text_clip.right() - galley.size().x
    } else {
        text_clip.left()
    };
    ui.painter().with_clip_rect(text_clip).galley(
        Pos2::new(tx, pr.center().y - galley.size().y / 2.0),
        galley,
        col,
    );
    if overflow {
        ui.interact(
            pr,
            ui.make_persistent_id(("depot_path_tip", i)),
            egui::Sense::hover(),
        )
        .on_hover_text(lp);
    }
    if Button::new("Browse", ButtonKind::Secondary)
        .icon(theme::icon::FOLDER)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(pr.right() + gap, y),
                Vec2::new(browse_w, px(ROW_H)),
            ),
            s,
        )
        .clicked()
    {
        app.projects_ui.spawn_pick(ctx, PickField::DepotLocal, i);
    }
    let running = matches!(app.projects_ui.scans.get(i), Some(ScanState::Running));
    if Button::new("Scan", ButtonKind::Secondary)
        .loading(running)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x + w - scan_w, y), Vec2::new(scan_w, px(ROW_H))),
            s,
        )
        .clicked()
        && !running
    {
        act = Some(DepotAct::Scan(i));
    }
    y += px(ROW_H + ROW_GAP);

    Input::new(&mut app.projects_ui.base.depots[i].depot_path)
        .hint(".")
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(120.0), px(OPT_H))),
            s,
        );
    kit::checkbox(
        ui,
        Rect::from_min_size(Pos2::new(x + px(140.0), y), Vec2::new(px(160.0), px(OPT_H))),
        &mut app.projects_ui.base.depots[i].recursive,
        "Recursive",
        s,
    );
    if let Some(ScanState::Done { files, bytes }) = app.projects_ui.scans.get(i).copied() {
        ui.painter().text(
            Pos2::new(x + w, y + px(OPT_H) / 2.0),
            Align2::RIGHT_CENTER,
            format!("{files} files  {}", human_size(bytes)),
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
    }
    y += px(OPT_H + ROW_GAP + 4.0);

    ui.painter().text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        "Exclusions",
        FontId::new(px(11.0), theme::family_label()),
        theme::INK_MUTED,
    );
    y += px(LABEL_H);
    for j in 0..n_excl {
        Input::new(&mut app.projects_ui.base.depots[i].exclusions[j])
            .hint("*.pdb")
            .show(
                ui,
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(220.0), px(EXCL_H))),
                s,
            );
        if Button::new("X", ButtonKind::Danger)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(x + px(228.0), y), Vec2::new(px(38.0), px(EXCL_H))),
                s,
            )
            .clicked()
        {
            act = Some(DepotAct::RemoveExcl(i, j));
        }
        y += px(EXCL_STEP);
    }
    if Button::new("+ Pattern", ButtonKind::Secondary)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(128.0), px(ADD_H))),
            s,
        )
        .clicked()
    {
        act = Some(DepotAct::AddExcl(i));
    }
    act
}
