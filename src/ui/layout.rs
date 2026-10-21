#![allow(dead_code)]

use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};

use crate::ui::theme;

pub struct Layout {
    pub scale: f32,
    pub rail: Rect,
    pub search: Rect,
    pub hero: Rect,
    pub hero_bite: Rect,
    pub prompt_bar: Rect,
    pub notch: Pos2,
    pub stack: Rect,
    pub choose: Rect,
    pub patch: Rect,
    pub header: Rect,
    pub content: Rect,
    pub toast_anchor: Pos2,
    pub show_right: bool,
}

pub fn compute(window: Rect) -> Layout {
    let s = theme::scale(window.size());
    let px = |v: f32| v * s;
    let m = px(24.0);
    let top = window.top() + ((window.height() - px(830.0)) * 0.5).max(px(16.0));
    let left = window.left() + m;
    let right = window.right() - m;
    let show_right = window.width() >= 900.0;

    let rail = Rect::from_min_size(pos2(left, top + px(120.0)), vec2(px(92.0), px(710.0)));
    let right_x = right - px(266.0);
    let search = Rect::from_min_size(pos2(right_x, top), vec2(px(266.0), px(68.0)));
    let stack = Rect::from_min_size(pos2(right_x, top + px(92.0)), vec2(px(266.0), px(442.0)));
    let patch = Rect::from_min_size(pos2(right_x, top + px(556.0)), vec2(px(266.0), px(274.0)));

    let hero_left = left + px(116.0);
    let hero_right = if show_right {
        right_x - px(24.0)
    } else {
        right
    };
    let hero = Rect::from_min_max(pos2(hero_left, top), pos2(hero_right, top + px(830.0)));

    let choose = Rect::from_min_size(
        pos2(hero.right() - px(388.0), hero.bottom() - px(250.0)),
        vec2(px(388.0), px(250.0)),
    );
    let hero_bite = Rect::from_min_max(
        pos2(choose.left() - px(24.0), choose.top() - px(24.0)),
        hero.max,
    );
    let prompt_bar = Rect::from_min_size(
        pos2(hero.left() + px(44.0), hero.bottom() - px(76.0)),
        vec2(px(278.0), px(48.0)),
    );
    let notch = pos2(hero.left() + px(30.0), hero.center().y);

    let header = Rect::from_min_size(pos2(hero_left, top), vec2(right - hero_left, px(68.0)));
    let content = Rect::from_min_max(
        pos2(hero_left, header.bottom() + px(24.0)),
        pos2(right, top + px(830.0)),
    );
    let toast_anchor = pos2(right - px(16.0), top + px(16.0));

    Layout {
        scale: s,
        rail,
        search,
        hero,
        hero_bite,
        prompt_bar,
        notch,
        stack,
        choose,
        patch,
        header,
        content,
        toast_anchor,
        show_right,
    }
}

impl Layout {
    pub fn translated(&self, d: Vec2) -> Layout {
        Layout {
            scale: self.scale,
            rail: self.rail.translate(d),
            search: self.search.translate(d),
            hero: self.hero.translate(d),
            hero_bite: self.hero_bite.translate(d),
            prompt_bar: self.prompt_bar.translate(d),
            notch: self.notch + d,
            stack: self.stack.translate(d),
            choose: self.choose.translate(d),
            patch: self.patch.translate(d),
            header: self.header.translate(d),
            content: self.content.translate(d),
            toast_anchor: self.toast_anchor + d,
            show_right: self.show_right,
        }
    }
}
