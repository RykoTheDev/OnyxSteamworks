use eframe::egui;

use crate::app::{App, Page};
use crate::ui::layout::Layout;
use crate::ui::{nav, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let idx = nav::index(Page::Pipelines);
    let tabs = nav::ITEMS[idx].sub_tabs;
    if let Some(t) = super::header(ui, layout, "PIPELINES", tabs, app.sub_tabs[idx]) {
        app.sub_tabs[idx] = t;
    }
    super::empty_state(
        ui,
        layout.content,
        theme::icon::CROP,
        "No pipelines yet",
        layout.scale,
    );
}
