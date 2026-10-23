#![allow(dead_code)]

use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};

use crate::ui::theme;

pub struct Layout {
    pub scale: f32,
    pub frame: Rect,
    pub rail: Rect,
    pub titlebar: Rect,
    pub header: Rect,
    pub content: Rect,
    pub toast_anchor: Pos2,
}

pub fn compute(window: Rect) -> Layout {
    let s = theme::scale(window.size());
    let px = |v: f32| v * s;
    let titlebar = Rect::from_min_size(
        window.min,
        vec2(window.width(), px(crate::ui::window::TITLEBAR_H)),
    );
    let rail_w = px(228.0);
    let rail = Rect::from_min_max(
        pos2(window.left(), titlebar.bottom()),
        pos2(window.left() + rail_w, window.bottom()),
    );
    let inset = px(8.0);
    let frame = Rect::from_min_max(
        pos2(rail.right(), titlebar.bottom()),
        pos2(window.right() - inset, window.bottom() - inset),
    );
    let header = Rect::from_min_size(frame.min, vec2(frame.width(), px(46.0)));
    let body = Rect::from_min_max(pos2(frame.left(), header.bottom()), frame.max);
    let content = Rect::from_min_max(
        pos2(body.left() + px(28.0), body.top() + px(22.0)),
        pos2(body.right() - px(28.0), body.bottom() - px(24.0)),
    );
    let toast_anchor = pos2(frame.right() - px(16.0), header.bottom() + px(12.0));
    Layout {
        scale: s,
        frame,
        rail,
        titlebar,
        header,
        content,
        toast_anchor,
    }
}

impl Layout {
    pub fn translated(&self, d: Vec2) -> Layout {
        Layout {
            scale: self.scale,
            frame: self.frame,
            rail: self.rail,
            titlebar: self.titlebar,
            header: self.header,
            content: self.content.translate(d),
            toast_anchor: self.toast_anchor + d,
        }
    }
}
