pub mod history;
pub mod home;
pub mod pipelines;
pub mod projects;
pub mod settings;

#[cfg(debug_assertions)]
pub mod anim_lab;

use eframe::egui::{self, Align2, FontId, Pos2, Rect};

use crate::app::{App, Page};
use crate::ui::bento::{chip, panel};
use crate::ui::layout::Layout;
use crate::ui::theme;

pub fn render(app: &mut App, ui: &mut egui::Ui, page: Page, layout: &Layout) {
    match page {
        Page::Home => home::show(app, ui, layout),
        Page::Projects => projects::show(app, ui, layout),
        Page::History => history::show(app, ui, layout),
        Page::Pipelines => pipelines::show(app, ui, layout),
        Page::Settings => settings::show(app, ui, layout),
    }
}

pub(crate) fn header(
    ui: &mut egui::Ui,
    layout: &Layout,
    title: &str,
    tabs: &[&str],
    active: usize,
) -> Option<usize> {
    let s = layout.scale;
    let px = |v: f32| theme::px(v, s);
    let rect = layout.header;
    panel::panel(
        &ui.painter().clone(),
        rect,
        theme::PANEL,
        rect.height() / 2.0,
    );
    ui.painter().text(
        Pos2::new(rect.left() + px(28.0), rect.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::new(px(14.0), theme::family_semibold()),
        theme::INK,
    );
    let mut clicked = None;
    let mut x = rect.left() + px(200.0);
    for (i, label) in tabs.iter().enumerate() {
        let size = chip::chip_size(ui, label, s);
        let cr = Rect::from_min_size(Pos2::new(x, rect.center().y - size.y / 2.0), size);
        if chip::chip(ui, cr, label, i == active, s).clicked() {
            clicked = Some(i);
        }
        x += size.x + px(8.0);
    }
    clicked
}

pub(crate) fn empty_state(ui: &mut egui::Ui, rect: Rect, glyph: char, message: &str, s: f32) {
    let px = |v: f32| theme::px(v, s);
    panel::panel(
        &ui.painter().clone(),
        rect,
        theme::PANEL,
        px(theme::RADIUS_PANEL_MIN),
    );
    let painter = ui.painter();
    let c = rect.center();
    painter.text(
        Pos2::new(c.x, c.y - px(18.0)),
        Align2::CENTER_CENTER,
        glyph.to_string(),
        theme::icon_font(px(40.0), false),
        theme::SEGMENT_IDLE,
    );
    painter.text(
        Pos2::new(c.x, c.y + px(24.0)),
        Align2::CENTER_CENTER,
        message,
        FontId::new(px(14.0), theme::family_medium()),
        theme::INK_MUTED,
    );
}
