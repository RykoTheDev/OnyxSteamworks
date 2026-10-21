use eframe::egui;

use crate::app::{App, Page};
use crate::ui::layout::Layout;
use crate::ui::{nav, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let idx = nav::index(Page::Settings);
    let tabs = nav::ITEMS[idx].sub_tabs;
    if let Some(t) = super::header(ui, layout, "SETTINGS", tabs, app.sub_tabs[idx]) {
        app.sub_tabs[idx] = t;
    }
    super::empty_state(
        ui,
        layout.content,
        theme::icon::GEAR,
        "Settings",
        layout.scale,
    );
}
