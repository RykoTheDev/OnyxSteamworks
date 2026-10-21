mod advanced;
mod doctor;

pub use advanced::AdvancedState;
pub use doctor::DoctorState;

use eframe::egui;

use crate::app::{App, Page};
use crate::ui::layout::Layout;
use crate::ui::{kit, nav, pages, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let idx = nav::index(Page::Settings);
    let tabs = nav::ITEMS[idx].sub_tabs;
    let mut active = app.sub_tabs[idx];
    let body = pages::hero_and_tabs(
        ui,
        layout.content,
        layout.scale,
        "SETTINGS",
        tabs,
        &mut active,
    );
    app.sub_tabs[idx] = active;

    if active == 3 {
        doctor::show(app, ui, body, layout.scale);
    } else if active == 4 {
        advanced::show(app, ui, body, layout.scale);
    } else {
        let (title, note) = match active {
            0 => (
                "General settings",
                "Defaults for builds and the review step will live here.",
            ),
            1 => (
                "Accounts",
                "Steam accounts and the remember-password toggle will live here.",
            ),
            _ => ("Notifications", "Build notifications will live here."),
        };
        kit::empty_state(ui, body, theme::icon::GEAR, title, note, layout.scale);
    }
}
