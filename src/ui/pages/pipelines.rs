use eframe::egui;

use crate::app::{App, Page};
use crate::ui::layout::Layout;
use crate::ui::{nav, pages, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let idx = nav::index(Page::Pipelines);
    let tabs = nav::ITEMS[idx].sub_tabs;
    let mut active = app.sub_tabs[idx];
    pages::simple_page(
        ui,
        layout,
        "PIPELINES",
        tabs,
        &mut active,
        theme::icon::FLOW,
        "No pipelines yet",
        "Chain build steps into a repeatable release pipeline.",
    );
    app.sub_tabs[idx] = active;
}
