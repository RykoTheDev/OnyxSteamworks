pub mod build;
pub mod history;
pub mod home;
pub mod pipelines;
pub mod projects;
pub mod settings;

use eframe::egui::{self, Pos2, Rect, Vec2};

use crate::app::{App, Page};
use crate::ui::layout::Layout;
use crate::ui::{kit, theme};

pub fn render(app: &mut App, ui: &mut egui::Ui, page: Page, layout: &Layout) {
    match page {
        Page::Home => home::show(app, ui, layout),
        Page::Projects => projects::show(app, ui, layout),
        Page::Build => build::show(app, ui, layout),
        Page::History => history::show(app, ui, layout),
        Page::Pipelines => pipelines::show(app, ui, layout),
        Page::Settings => settings::show(app, ui, layout),
    }
}

pub(crate) fn hero_and_tabs(
    ui: &mut egui::Ui,
    content: Rect,
    scale: f32,
    title: &str,
    tabs: &[&str],
    active: &mut usize,
) -> Rect {
    let px = |v: f32| theme::px(v, scale);
    kit::hero(&ui.painter().clone(), content, title, scale);
    let mut top = content.top() + px(96.0);
    if !tabs.is_empty() {
        let font = egui::FontId::new(px(12.0), theme::family_semibold());
        let widest = tabs
            .iter()
            .map(|t| {
                ui.painter()
                    .layout_no_wrap(t.to_string(), font.clone(), theme::INK)
                    .size()
                    .x
            })
            .fold(0.0_f32, f32::max);
        let seg = (widest + px(28.0)).max(px(84.0));
        let tw = (seg * tabs.len() as f32).min(content.width());
        let tr = Rect::from_min_size(Pos2::new(content.left(), top), Vec2::new(tw, px(34.0)));
        kit::tabs(ui, tr, tabs, active, scale);
        top = tr.bottom() + px(20.0);
    }
    Rect::from_min_max(Pos2::new(content.left(), top), content.max)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn simple_page(
    ui: &mut egui::Ui,
    layout: &Layout,
    title: &str,
    tabs: &[&str],
    active: &mut usize,
    glyph: char,
    empty_title: &str,
    empty_body: &str,
) {
    let body = hero_and_tabs(ui, layout.content, layout.scale, title, tabs, active);
    kit::empty_state(ui, body, glyph, empty_title, empty_body, layout.scale);
}
