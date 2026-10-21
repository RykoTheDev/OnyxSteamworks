mod accounts;
mod advanced;
mod developer;
mod doctor;
mod general;
mod notifications;

pub use accounts::AccountsState;
pub use advanced::AdvancedState;
pub use developer::DeveloperState;
pub use doctor::DoctorState;
pub use general::GeneralState;
pub use notifications::NotificationsState;

use eframe::egui;

use crate::app::{App, Page};
use crate::ui::layout::Layout;
use crate::ui::{nav, pages};

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

    match active {
        1 => accounts::show(app, ui, body, layout.scale),
        2 => notifications::show(app, ui, body, layout.scale),
        3 => doctor::show(app, ui, body, layout.scale),
        4 => advanced::show(app, ui, body, layout.scale),
        _ => general::show(app, ui, body, layout.scale),
    }
}
