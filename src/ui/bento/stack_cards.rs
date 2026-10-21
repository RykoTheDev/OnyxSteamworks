#![allow(dead_code)]

use std::f32::consts::FRAC_PI_2;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2, epaint::TextShape};

use super::image;
use crate::ui::theme;

pub enum CardContent<'a> {
    Image(&'a str),
    Letter(char),
}

pub struct StackCard<'a> {
    pub number: &'a str,
    pub content: CardContent<'a>,
}

pub fn stack_cards(ui: &mut egui::Ui, rect: Rect, cards: &[StackCard<'_>], s: f32) {
    let px = |v: f32| theme::px(v, s);
    let card_h = px(170.0);
    let overlap = px(34.0);
    let n = cards.len();
    for draw in 0..n {
        let i = n - 1 - draw;
        let top = rect.top() + i as f32 * (card_h - overlap);
        let card_rect =
            Rect::from_min_size(Pos2::new(rect.left(), top), Vec2::new(rect.width(), card_h));
        match &cards[i].content {
            CardContent::Image(uri) => {
                image::image_or_placeholder(
                    ui,
                    card_rect,
                    Some(uri),
                    ' ',
                    px(theme::RADIUS_CARD),
                    s,
                );
            }
            CardContent::Letter(letter) => {
                image::image_or_placeholder(
                    ui,
                    card_rect,
                    None,
                    *letter,
                    px(theme::RADIUS_CARD),
                    s,
                );
            }
        }
        let tab = Rect::from_min_size(
            Pos2::new(card_rect.left() + px(12.0), card_rect.center().y - px(26.0)),
            Vec2::new(px(28.0), px(52.0)),
        );
        let painter = ui.painter();
        painter.rect_filled(
            tab,
            theme::corner(px(14.0)),
            theme::with_alpha(theme::PANEL, 210),
        );
        painter.rect_stroke(
            tab,
            theme::corner(px(14.0)),
            Stroke::new(1.5, theme::INK),
            StrokeKind::Inside,
        );
        let galley = painter.layout_no_wrap(
            cards[i].number.to_string(),
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK,
        );
        painter.add(
            TextShape::new(tab.center(), galley, theme::INK)
                .with_angle_and_anchor(-FRAC_PI_2, Align2::CENTER_CENTER),
        );
    }
}
