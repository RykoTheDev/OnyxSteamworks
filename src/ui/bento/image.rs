#![allow(dead_code)]

use eframe::egui::{self, Align2, FontId, Rect};

use super::panel;
use crate::ui::theme;

pub fn image_or_placeholder(
    ui: &mut egui::Ui,
    rect: Rect,
    uri: Option<&str>,
    letter: char,
    radius_px: f32,
    s: f32,
) {
    match uri {
        Some(u) if !u.is_empty() => {
            egui::Image::from_uri(u.to_owned())
                .maintain_aspect_ratio(false)
                .fit_to_exact_size(rect.size())
                .corner_radius(theme::corner(radius_px))
                .paint_at(ui, rect);
        }
        _ => {
            let painter = ui.painter();
            panel::panel(painter, rect, theme::PANEL_ALT, radius_px);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                letter.to_uppercase().to_string(),
                FontId::new(theme::px(18.0, s), theme::family_semibold()),
                theme::INK_MUTED,
            );
        }
    }
}
