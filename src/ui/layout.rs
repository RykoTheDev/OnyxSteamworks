#![allow(dead_code)]

use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};

use crate::ui::theme;

pub struct Layout {
    pub scale: f32,
    pub frame: Rect,
    pub rail: Rect,
    pub content: Rect,
    pub toast_anchor: Pos2,
}

pub fn compute(window: Rect) -> Layout {
    let s = theme::scale(window.size());
    let px = |v: f32| v * s;
    let margin = px(10.0);
    let frame = window.shrink(margin);
    let rail_w = px(62.0);
    let rail = Rect::from_min_size(frame.min, vec2(rail_w, frame.height()));
    let content_outer = Rect::from_min_max(pos2(rail.right(), frame.top()), frame.max);
    let content = content_outer.shrink2(vec2(px(30.0), px(26.0)));
    let toast_anchor = pos2(content.right(), content.top());
    Layout {
        scale: s,
        frame,
        rail,
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
            content: self.content.translate(d),
            toast_anchor: self.toast_anchor + d,
        }
    }
}
