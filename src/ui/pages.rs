pub mod automation;
pub mod build;
pub mod history;
pub mod projects;
pub mod settings;

use eframe::egui::{self, Rect};

use crate::app::{App, Page};
use crate::ui::layout::Layout;

pub fn render(app: &mut App, ui: &mut egui::Ui, page: Page, layout: &Layout) {
    match page {
        Page::Projects => projects::show(app, ui, layout),
        Page::Build => build::show(app, ui, layout),
        Page::History => history::show(app, ui, layout),
        Page::Automation => automation::show(app, ui, layout),
        Page::Settings => settings::show(app, ui, layout),
    }
}

pub(crate) fn hero_and_tabs(
    _ui: &mut egui::Ui,
    content: Rect,
    _scale: f32,
    _title: &str,
    _tabs: &[&str],
    _active: &mut usize,
) -> Rect {
    content
}
