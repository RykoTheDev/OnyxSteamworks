use eframe::egui::{self, Pos2, Rect, Vec2};

use super::label;
use crate::app::App;
use crate::ui::bento::pill_button::{PillButton, PillKind};
use crate::ui::bento::{checkbox, icon_button, text_input, toggle};
use crate::ui::theme;

pub(super) fn form_demos(app: &mut App, ui: &mut egui::Ui, s: f32, left: f32, top: f32, w: f32) {
    let px = |v: f32| theme::px(v, s);
    let rc =
        |x: f32, y: f32, ww: f32, hh: f32| Rect::from_min_size(Pos2::new(x, y), Vec2::new(ww, hh));
    let mut y = top;

    y = label(&ui.painter().clone(), left, y, px(12.0), "BUTTONS");
    PillButton::new("SAVE", PillKind::Primary)
        .icon(theme::icon::DOWNLOAD)
        .show(ui, rc(left, y, px(130.0), px(40.0)), s);
    PillButton::new("CANCEL", PillKind::Secondary).show(
        ui,
        rc(left + px(142.0), y, px(120.0), px(40.0)),
        s,
    );
    PillButton::new("DELETE", PillKind::Danger).show(
        ui,
        rc(left + px(274.0), y, px(120.0), px(40.0)),
        s,
    );
    y += px(52.0);

    y = label(&ui.painter().clone(), left, y, px(12.0), "STATES");
    PillButton::new("SAVING", PillKind::Primary)
        .loading(true)
        .show(ui, rc(left, y, px(130.0), px(40.0)), s);
    PillButton::new("PUBLISH", PillKind::Primary)
        .enabled(false)
        .disabled_reason("Fix errors first")
        .show(ui, rc(left + px(142.0), y, px(140.0), px(40.0)), s);
    y += px(52.0);

    y = label(
        &ui.painter().clone(),
        left,
        y,
        px(12.0),
        "ICON / TOGGLE / CHECK",
    );
    icon_button::icon_button(
        ui,
        Pos2::new(left + px(22.0), y + px(22.0)),
        theme::icon::HEART,
        s,
    );
    toggle::toggle(
        ui,
        rc(left + px(70.0), y + px(10.0), px(44.0), px(24.0)),
        &mut app.lab.toggle_on,
        s,
    );
    checkbox::checkbox(
        ui,
        rc(left + px(150.0), y + px(11.0), px(22.0), px(22.0)),
        &mut app.lab.checked,
        s,
    );
    y += px(56.0);

    y = label(&ui.painter().clone(), left, y, px(12.0), "TEXT INPUT");
    text_input::TextInput::new(&mut app.lab.input)
        .hint("Project name")
        .helper("Shown in the launcher")
        .show(ui, rc(left, y, w.min(px(300.0)), px(40.0)), s);
    y += px(58.0);
    text_input::TextInput::new(&mut app.lab.secret)
        .hint("Steam password")
        .mask(true)
        .show(ui, rc(left, y, w.min(px(300.0)), px(40.0)), s);
    y += px(50.0);
    let err = app.lab.error_text.is_empty();
    text_input::TextInput::new(&mut app.lab.error_text)
        .hint("Required field")
        .error(err)
        .helper("This field is required")
        .show(ui, rc(left, y, w.min(px(300.0)), px(40.0)), s);
}
