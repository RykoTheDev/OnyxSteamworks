#![allow(dead_code)]

use eframe::egui::{Color32, Painter, Rect};

use crate::ui::theme;

pub fn panel(painter: &Painter, rect: Rect, fill: Color32, radius_px: f32) {
    painter.rect_filled(rect, theme::corner(radius_px), fill);
}

pub fn cutout(painter: &Painter, rect: Rect, radius_px: f32) {
    painter.rect_filled(rect, theme::corner(radius_px), theme::BG);
}
