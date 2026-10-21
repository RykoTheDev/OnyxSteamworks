use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Vec2};

use crate::app::App;
use crate::model::{BuildRecord, Project};
use crate::ui::pages::history;
use crate::ui::{anim, kit, theme};

pub(super) fn stacked(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    records: &[BuildRecord],
    total: usize,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let card_h = px(58.0);
    let gap = px(10.0);
    for (i, record) in records.iter().enumerate() {
        let top = rect.top() + i as f32 * (card_h + gap);
        if top + card_h > rect.bottom() {
            break;
        }
        let cr = Rect::from_min_size(Pos2::new(rect.left(), top), Vec2::new(rect.width(), card_h));
        let id = ui.make_persistent_id(("homecard", &record.id));
        let resp = ui.interact(cr, id, Sense::click());
        let ctx = ui.ctx().clone();
        let hover = anim::anim_bool(
            &ctx,
            id.with("h"),
            resp.hovered(),
            0.12,
            anim::ease_out_cubic,
        );
        kit::bordered(
            ui.painter(),
            cr,
            theme::lerp_color(theme::SURFACE_2, theme::FIELD, hover),
            theme::LINE,
            px(theme::RADIUS_CARD),
        );

        let tab = Rect::from_min_size(
            Pos2::new(cr.left() + px(12.0), cr.center().y - px(16.0)),
            Vec2::new(px(34.0), px(32.0)),
        );
        kit::bordered(
            ui.painter(),
            tab,
            theme::FIELD,
            theme::LINE,
            px(theme::RADIUS_FIELD),
        );
        ui.painter().text(
            tab.center(),
            Align2::CENTER_CENTER,
            format!("{}", total - i),
            FontId::new(px(14.0), theme::family_semibold()),
            theme::INK,
        );

        let branch = if record.branch.is_empty() {
            "default"
        } else {
            record.branch.as_str()
        };
        let bsize = kit::chip_size(ui, branch, s);
        let brect = Rect::from_min_size(
            Pos2::new(tab.right() + px(12.0), cr.center().y - bsize.y * 0.5),
            bsize,
        );
        kit::chip(ui, brect, branch, false, s);

        let rt = history::rel_time(record.timestamp);
        let rt_font = FontId::new(px(12.0), theme::family_medium());
        let rt_w = ui
            .painter()
            .layout_no_wrap(rt.clone(), rt_font.clone(), theme::INK_MUTED)
            .size()
            .x;
        let rt_x = cr.right() - px(16.0);
        ui.painter().text(
            Pos2::new(rt_x, cr.center().y),
            Align2::RIGHT_CENTER,
            &rt,
            rt_font,
            theme::INK_MUTED,
        );

        let dot_x = rt_x - rt_w - px(14.0);
        ui.painter().circle_filled(
            Pos2::new(dot_x, cr.center().y),
            px(4.0),
            history::status_color(record.status),
        );

        let size = app
            .home_ui
            .size_label(&record.id)
            .unwrap_or_else(|| "…".to_string());
        ui.painter().text(
            Pos2::new(dot_x - px(14.0), cr.center().y),
            Align2::RIGHT_CENTER,
            size,
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_DIM,
        );

        if resp.clicked() {
            app.history_ui.open_detail(record.clone());
            super::open_history(app, 0);
        }
    }
}

pub(super) fn choose_branch(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    project: &Project,
    s: f32,
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
    kit::label(
        &ui.painter().clone(),
        Pos2::new(x, rect.top() + pad),
        "CHOOSE BRANCH",
        s,
    );

    let mut cx = x;
    let mut cy = rect.top() + px(46.0);
    if project.branches.is_empty() {
        ui.painter().text(
            Pos2::new(x, cy + px(6.0)),
            Align2::LEFT_TOP,
            "default branch only",
            FontId::new(px(12.5), theme::family_medium()),
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
            if cx + size.x > rect.right() - pad {
                cx = x;
                cy += px(36.0);
            }
            let r = Rect::from_min_size(Pos2::new(cx, cy), size);
            if kit::chip(ui, r, label, i == app.build_ui.branch_sel, s).clicked() {
                app.build_ui.branch_sel = i;
            }
            cx += size.x + px(8.0);
        }
    }

    let oy = rect.bottom() - px(78.0);
    kit::label(
        &ui.painter().clone(),
        Pos2::new(x, oy - px(22.0)),
        "OPTIONS",
        s,
    );
    let mut ox = x;
    ox = option_chip(
        ui,
        &mut ox,
        oy,
        "PREVIEW",
        &mut app.build_ui.preview_only,
        s,
    );
    ox = option_chip(ui, &mut ox, oy, "BACKUP", &mut app.build_ui.backup, s);
    option_chip(ui, &mut ox, oy, "VERIFY", &mut app.build_ui.verify, s);

    let seg = Rect::from_min_size(
        Pos2::new(x, rect.bottom() - px(24.0)),
        Vec2::new(rect.width() - pad * 2.0, px(6.0)),
    );
    kit::segment_progress(ui, seg, 5, app.pipeline_ui.segment_fill(5), s);
}

fn option_chip(
    ui: &mut egui::Ui,
    ox: &mut f32,
    oy: f32,
    label: &str,
    flag: &mut bool,
    s: f32,
) -> f32 {
    let px = |v: f32| theme::px(v, s);
    let size = kit::chip_size(ui, label, s);
    let r = Rect::from_min_size(Pos2::new(*ox, oy), size);
    if kit::chip(ui, r, label, *flag, s).clicked() {
        *flag = !*flag;
    }
    *ox += size.x + px(8.0);
    *ox
}

pub(super) fn patch_notes(app: &mut App, ui: &mut egui::Ui, rect: Rect, project: &Project, s: f32) {
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
    kit::label(
        &ui.painter().clone(),
        Pos2::new(x, rect.top() + pad),
        "PATCH NOTES",
        s,
    );

    let notes = app
        .history_ui
        .latest_for(&project.id)
        .and_then(|r| r.notes.clone())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            "No release notes yet. They will appear here once a build saves them.".to_string()
        });
    let galley = ui.painter().layout(
        notes,
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_DIM,
        rect.width() - pad * 2.0,
    );
    ui.painter()
        .galley(Pos2::new(x, rect.top() + px(48.0)), galley, theme::INK_DIM);

    let btn = Rect::from_min_size(
        Pos2::new(
            rect.right() - pad - px(40.0),
            rect.bottom() - pad - px(40.0),
        ),
        Vec2::splat(px(40.0)),
    );
    if super::circle_button(ui, btn, theme::icon::HISTORY, false, s).clicked() {
        super::open_history(app, 2);
    }
}
