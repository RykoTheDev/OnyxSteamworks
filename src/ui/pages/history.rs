use eframe::egui;

use crate::app::{App, Page};
use crate::ui::layout::Layout;
use crate::ui::{nav, pages, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let idx = nav::index(Page::History);
    let tabs = nav::ITEMS[idx].sub_tabs;
    let mut active = app.sub_tabs[idx];
    pages::simple_page(
        ui,
        layout,
        "HISTORY",
        tabs,
        &mut active,
        theme::icon::HISTORY,
        "No builds yet",
        "Finished builds and their logs will appear here.",
    );
    app.sub_tabs[idx] = active;
}
