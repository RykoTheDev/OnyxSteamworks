use eframe::egui::{self, Align2, Pos2, Rect, Sense, Vec2};

use crate::app::Page;
use crate::ui::{anim, nav, theme};

pub enum RailAction {
    Navigate(Page),
    Logo,
    Logout,
    Avatar,
}

pub fn rail(ui: &mut egui::Ui, rect: Rect, active: Page, avatar: char, s: f32) -> Option<RailAction> {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let mut action = None;
    let cx = rect.center().x;

    let logo_c = Pos2::new(cx, rect.top() + px(30.0));
    let logo_hit = Rect::from_center_size(logo_c, Vec2::splat(px(36.0)));
    let logo_resp = ui.interact(logo_hit, ui.make_persistent_id("rail_logo"), Sense::click());
    let logo_hover = anim::anim_bool(&ctx, ui.make_persistent_id("rail_logo"), logo_resp.hovered(), 0.14, anim::ease_out_cubic);
    ui.painter().text(
        logo_c,
        Align2::CENTER_CENTER,
        theme::icon::SPARKLE.to_string(),
        theme::icon_font(px(22.0) * (1.0 + 0.04 * logo_hover), true),
        theme::INK,
    );
    if logo_resp.clicked() {
        action = Some(RailAction::Logo);
    }

    let first = rect.top() + px(86.0);
    let step = px(58.0);
    for (i, item) in nav::ITEMS.iter().enumerate() {
        let c = Pos2::new(cx, first + i as f32 * step);
        let slot = Rect::from_center_size(c, Vec2::new(px(44.0), px(42.0)));
        let id = ui.make_persistent_id(("rail_slot", item.page));
        let resp = ui.interact(slot, id, Sense::click());
        if resp.clicked() {
            action = Some(RailAction::Navigate(item.page));
        }
        let t = anim::anim_bool(&ctx, id.with("active"), item.page == active, 0.18, anim::ease_out_cubic);
        let hover = anim::anim_bool(&ctx, id.with("hover"), resp.hovered(), 0.12, anim::ease_out_cubic);
        if t > 0.01 || hover > 0.01 {
            let hi = theme::lerp_color(theme::SURFACE, theme::SURFACE_2, (t + hover * 0.5).min(1.0));
            ui.painter().rect_filled(slot, theme::corner(px(theme::RADIUS_FIELD)), hi);
        }
        let col = theme::lerp_color(
            theme::lerp_color(theme::INK_MUTED, theme::INK_DIM, hover),
            theme::INK,
            t,
        );
        ui.painter().text(
            c,
            Align2::CENTER_CENTER,
            item.glyph.to_string(),
            theme::icon_font(px(21.0), false),
            anim::with_opacity(col, 1.0 - t),
        );
        ui.painter().text(
            c,
            Align2::CENTER_CENTER,
            item.glyph.to_string(),
            theme::icon_font(px(21.0), true),
            anim::with_opacity(theme::INK, t),
        );
    }

    let avatar_r = px(16.0);
    let avatar_c = Pos2::new(cx, rect.bottom() - px(26.0));
    let avatar_hit = Rect::from_center_size(avatar_c, Vec2::splat(avatar_r * 2.0));
    crate::ui::kit::image_or_placeholder(ui, avatar_hit, None, avatar, avatar_r, s);
    let avatar_resp = ui.interact(avatar_hit, ui.make_persistent_id("rail_avatar"), Sense::click());
    if avatar_resp.clicked() {
        action = Some(RailAction::Avatar);
    }

    let logout_c = Pos2::new(cx, avatar_c.y - px(44.0));
    let logout_hit = Rect::from_center_size(logout_c, Vec2::splat(px(40.0)));
    let logout_resp = ui.interact(logout_hit, ui.make_persistent_id("rail_logout"), Sense::click());
    if logout_resp.clicked() {
        action = Some(RailAction::Logout);
    }
    let logout_hover = anim::anim_bool(&ctx, ui.make_persistent_id("rail_logout"), logout_resp.hovered(), 0.14, anim::ease_out_cubic);
    ui.painter().text(
        logout_c,
        Align2::CENTER_CENTER,
        theme::icon::SIGN_OUT.to_string(),
        theme::icon_font(px(19.0), false),
        theme::lerp_color(theme::INK_MUTED, theme::INK, logout_hover),
    );

    action
}
