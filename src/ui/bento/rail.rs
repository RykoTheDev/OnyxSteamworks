#![allow(dead_code)]

use eframe::egui::{self, Align2, Pos2, Rect, Sense, Vec2};

use super::{image, panel};
use crate::app::Page;
use crate::ui::{anim, nav, theme};

pub enum RailAction {
    Navigate(Page),
    Logo,
    Logout,
}

pub fn rail(ui: &mut egui::Ui, rect: Rect, active: Page, s: f32) -> Option<RailAction> {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let mut action = None;

    let painter = ui.painter().clone();
    panel::panel(&painter, rect, theme::PANEL, rect.width() / 2.0);

    let cx = rect.center().x;
    let logo_c = Pos2::new(cx, rect.top() - px(72.0));
    let logo_hit = Rect::from_center_size(logo_c, Vec2::splat(px(96.0)));
    let logo_resp = ui.interact(logo_hit, ui.make_persistent_id("rail_logo"), Sense::click());
    let logo_hover = anim::anim_bool(
        &ctx,
        ui.make_persistent_id("rail_logo"),
        logo_resp.hovered(),
        0.14,
        anim::ease_out_cubic,
    );
    painter.circle_filled(logo_c, px(48.0) * (1.0 + 0.03 * logo_hover), theme::PANEL);
    painter.text(
        logo_c,
        Align2::CENTER_CENTER,
        theme::icon::SPARKLE.to_string(),
        theme::icon_font(px(34.0), true),
        theme::INK,
    );
    if logo_resp.clicked() {
        action = Some(RailAction::Logo);
    }

    for (i, item) in nav::ITEMS.iter().enumerate() {
        let c = Pos2::new(cx, rect.top() + px(52.0) + i as f32 * px(68.0));
        let slot = Rect::from_center_size(c, Vec2::splat(px(44.0)));
        let id = ui.make_persistent_id(("rail_slot", item.page));
        let resp = ui.interact(slot, id, Sense::click());
        if resp.clicked() {
            action = Some(RailAction::Navigate(item.page));
        }
        let t = anim::anim_bool(
            &ctx,
            id.with("active"),
            item.page == active,
            0.2,
            anim::ease_out_cubic,
        );
        let hover = anim::anim_bool(
            &ctx,
            id.with("hover"),
            resp.hovered(),
            0.14,
            anim::ease_out_cubic,
        );
        let size = px(26.0) * (1.0 + 0.03 * hover);
        painter.text(
            c,
            Align2::CENTER_CENTER,
            item.glyph.to_string(),
            theme::icon_font(size, false),
            anim::with_opacity(theme::INK, 1.0 - t),
        );
        painter.text(
            c,
            Align2::CENTER_CENTER,
            item.glyph.to_string(),
            theme::icon_font(size, true),
            anim::with_opacity(theme::INK, t),
        );
    }

    let avatar_r = px(38.0);
    let avatar_c = Pos2::new(cx, rect.bottom() - px(8.0) - avatar_r);
    let logout_c = Pos2::new(cx, avatar_c.y - avatar_r - px(28.0));
    let logout_hit = Rect::from_center_size(logout_c, Vec2::splat(px(40.0)));
    let logout_resp = ui.interact(
        logout_hit,
        ui.make_persistent_id("rail_logout"),
        Sense::click(),
    );
    if logout_resp.clicked() {
        action = Some(RailAction::Logout);
    }
    let logout_hover = anim::anim_bool(
        &ctx,
        ui.make_persistent_id("rail_logout"),
        logout_resp.hovered(),
        0.14,
        anim::ease_out_cubic,
    );
    painter.text(
        logout_c,
        Align2::CENTER_CENTER,
        theme::icon::SIGN_OUT.to_string(),
        theme::icon_font(px(24.0) * (1.0 + 0.03 * logout_hover), false),
        theme::INK,
    );

    let avatar_rect = Rect::from_center_size(avatar_c, Vec2::splat(avatar_r * 2.0));
    image::image_or_placeholder(ui, avatar_rect, None, 'S', avatar_r, s);

    action
}
