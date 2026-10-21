use eframe::egui::{self, Pos2, Rect, Vec2};

use super::label;
use crate::app::App;
use crate::ui::bento::pill_button::{PillButton, PillKind};
use crate::ui::bento::toast::ToastLevel;
use crate::ui::bento::{dropdown, modal, progress, tabs};
use crate::ui::theme;

pub(super) fn feedback_demos(
    app: &mut App,
    ui: &mut egui::Ui,
    s: f32,
    left: f32,
    top: f32,
    _w: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let rc =
        |x: f32, y: f32, ww: f32, hh: f32| Rect::from_min_size(Pos2::new(x, y), Vec2::new(ww, hh));
    let colw = px(360.0);
    let right = left + colw + px(32.0);
    let mut y = top;

    y = label(&ui.painter().clone(), left, y, px(12.0), "TABS");
    tabs::tabs(
        ui,
        rc(left, y, colw, px(40.0)),
        &["BUILDS", "BACKUPS", "NOTES"],
        &mut app.lab.tab_active,
        s,
    );
    y += px(64.0);

    y = label(&ui.painter().clone(), left, y, px(12.0), "PROGRESS BAR");
    progress::progress_bar(ui, rc(left, y, colw, px(10.0)), Some(app.lab.progress), s);
    y += px(30.0);
    progress::progress_bar(ui, rc(left, y, colw, px(10.0)), None, s);
    y += px(30.0);
    if PillButton::new("STEP", PillKind::Secondary)
        .show(ui, rc(left, y, px(120.0), px(36.0)), s)
        .clicked()
    {
        app.lab.progress = if app.lab.progress >= 1.0 {
            0.0
        } else {
            app.lab.progress + 0.25
        };
    }
    progress::progress_ring(
        ui,
        Pos2::new(left + px(200.0), y + px(18.0)),
        px(22.0),
        app.lab.progress,
        s,
    );
    y += px(60.0);

    y = label(&ui.painter().clone(), left, y, px(12.0), "MODAL + TOOLTIP");
    if PillButton::new("OPEN MODAL", PillKind::Primary)
        .show(ui, rc(left, y, px(150.0), px(40.0)), s)
        .clicked()
    {
        app.lab.modal_open = true;
    }
    let tip = PillButton::new("HOVER ME", PillKind::Secondary).show(
        ui,
        rc(left + px(166.0), y, px(150.0), px(40.0)),
        s,
    );
    modal::tooltip(tip, "A black pill tooltip");

    let mut y2 = top;
    y2 = label(&ui.painter().clone(), right, y2, px(12.0), "DROPDOWN");
    dropdown::dropdown(
        ui,
        rc(right, y2, px(220.0), px(40.0)),
        &["Dev", "Beta", "Live"],
        &mut app.lab.dropdown_sel,
        s,
    );
    y2 += px(64.0);

    label(&ui.painter().clone(), right, y2, px(12.0), "TOASTS");
    y2 += px(18.0);
    if PillButton::new("SUCCESS", PillKind::Secondary)
        .show(ui, rc(right, y2, px(110.0), px(36.0)), s)
        .clicked()
    {
        app.lab.toasts.push(ToastLevel::Success, "Build uploaded");
    }
    if PillButton::new("WARNING", PillKind::Secondary)
        .show(ui, rc(right + px(118.0), y2, px(110.0), px(36.0)), s)
        .clicked()
    {
        app.lab.toasts.push(ToastLevel::Warning, "Low disk space");
    }
    if PillButton::new("ERROR", PillKind::Danger)
        .show(ui, rc(right + px(236.0), y2, px(110.0), px(36.0)), s)
        .clicked()
    {
        app.lab.toasts.push(ToastLevel::Error, "SteamCMD failed");
    }

    let anchor = Pos2::new(
        ui.max_rect().right() - px(24.0),
        ui.max_rect().top() + px(90.0),
    );
    app.lab.toasts.show(ui, anchor, s);

    let modal_id = ui.make_persistent_id("lab_modal");
    modal::modal(
        ui,
        modal_id,
        &mut app.lab.modal_open,
        Vec2::new(px(380.0), px(190.0)),
        s,
        |ui| {
            let r = ui.max_rect();
            let px = |v: f32| theme::px(v, s);
            ui.painter().text(
                Pos2::new(r.left() + px(28.0), r.top() + px(30.0)),
                egui::Align2::LEFT_TOP,
                "CONFIRM ACTION",
                egui::FontId::new(px(14.0), theme::family_semibold()),
                theme::INK,
            );
            ui.painter().text(
                Pos2::new(r.left() + px(28.0), r.top() + px(66.0)),
                egui::Align2::LEFT_TOP,
                "This is a modal with a small overshoot.",
                egui::FontId::new(px(13.0), theme::family_medium()),
                theme::INK_MUTED,
            );
            PillButton::new("CLOSE", PillKind::Primary)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(
                            r.right() - px(28.0) - px(140.0),
                            r.bottom() - px(28.0) - px(40.0),
                        ),
                        Vec2::new(px(140.0), px(40.0)),
                    ),
                    s,
                )
                .clicked()
        },
    );
}
