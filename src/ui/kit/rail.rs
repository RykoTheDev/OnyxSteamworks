use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::app::Page;
use crate::ui::{anim, nav, theme};

pub enum RailAction {
    Navigate(Page),
    SubTab(Page, usize),
    Search,
    Logo,
    Avatar,
}

pub struct RailState<'a> {
    pub active: Page,
    pub sub_tabs: &'a [usize],
    pub avatar: char,
    pub account: &'a str,
}

pub fn account_row(rect: Rect, s: f32) -> Rect {
    let px = |v: f32| theme::px(v, s);
    Rect::from_min_max(
        Pos2::new(rect.left() + px(10.0), rect.bottom() - px(46.0)),
        Pos2::new(rect.right() - px(10.0), rect.bottom() - px(14.0)),
    )
}

fn row_fill(ui: &egui::Ui, row: Rect, active: f32, hover: f32, s: f32) {
    if active < 0.01 && hover < 0.01 {
        return;
    }
    let fill = theme::lerp_color(
        theme::with_alpha(theme::FIELD, (hover * 255.0) as u8),
        theme::FIELD_HI,
        active,
    );
    ui.painter()
        .rect_filled(row, theme::corner(theme::px(theme::RADIUS_FIELD, s)), fill);
}

pub fn rail(ui: &mut egui::Ui, rect: Rect, st: RailState<'_>, s: f32) -> Option<RailAction> {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let mut action = None;
    let left = rect.left() + px(10.0);
    let width = rect.width() - px(20.0);
    let row_h = px(30.0);

    let ws = Rect::from_min_size(
        Pos2::new(left, rect.top() + px(12.0)),
        Vec2::new(width, px(34.0)),
    );
    let ws_id = ui.make_persistent_id("rail_logo");
    let ws_resp = ui.interact(ws, ws_id, Sense::click());
    let ws_hover = anim::anim_bool(&ctx, ws_id, ws_resp.hovered(), 0.12, anim::ease_out_cubic);
    row_fill(ui, ws, 0.0, ws_hover, s);
    let mark = Rect::from_center_size(
        Pos2::new(ws.left() + px(18.0), ws.center().y),
        Vec2::splat(px(22.0)),
    );
    ui.painter()
        .rect_filled(mark, theme::corner(px(6.0)), theme::ACCENT);
    ui.painter().text(
        mark.center(),
        Align2::CENTER_CENTER,
        theme::icon::SPARKLE.to_string(),
        theme::icon_font(px(13.0), true),
        theme::PRIMARY_INK,
    );
    ui.painter().text(
        Pos2::new(mark.right() + px(10.0), ws.center().y),
        Align2::LEFT_CENTER,
        "Onyx",
        FontId::new(px(14.0), theme::family_semibold()),
        theme::INK,
    );
    ui.painter().text(
        Pos2::new(ws.right() - px(10.0), ws.center().y),
        Align2::RIGHT_CENTER,
        theme::icon::CARET_DOWN.to_string(),
        theme::icon_font(px(11.0), false),
        theme::INK_FAINT,
    );
    if ws_resp.clicked() {
        action = Some(RailAction::Logo);
    }

    let search = Rect::from_min_size(
        Pos2::new(left, ws.bottom() + px(10.0)),
        Vec2::new(width, row_h),
    );
    let sid = ui.make_persistent_id("search_pill");
    let sresp = ui.interact(search, sid, Sense::click());
    let shover = anim::anim_bool(&ctx, sid, sresp.hovered(), 0.12, anim::ease_out_cubic);
    ui.painter().rect_filled(
        search,
        theme::corner(px(theme::RADIUS_FIELD)),
        theme::lerp_color(theme::SURFACE, theme::FIELD, shover),
    );
    ui.painter().rect_stroke(
        search,
        theme::corner(px(theme::RADIUS_FIELD)),
        Stroke::new(1.0, theme::LINE_SOFT),
        StrokeKind::Inside,
    );
    ui.painter().text(
        Pos2::new(search.left() + px(18.0), search.center().y),
        Align2::CENTER_CENTER,
        theme::icon::MAGNIFYING_GLASS.to_string(),
        theme::icon_font(px(14.0), false),
        theme::INK_MUTED,
    );
    ui.painter().text(
        Pos2::new(search.left() + px(32.0), search.center().y),
        Align2::LEFT_CENTER,
        "Search",
        FontId::new(px(13.0), theme::family_medium()),
        theme::lerp_color(theme::INK_MUTED, theme::INK_DIM, shover),
    );
    kbd(
        ui,
        Pos2::new(search.right() - px(8.0), search.center().y),
        "Ctrl K",
        s,
    );
    if sresp.clicked() {
        action = Some(RailAction::Search);
    }

    ui.painter().text(
        Pos2::new(left + px(8.0), search.bottom() + px(22.0)),
        Align2::LEFT_CENTER,
        "Workspace",
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK_FAINT,
    );

    let mut y = search.bottom() + px(34.0);
    for (i, item) in nav::ITEMS.iter().enumerate() {
        let row = Rect::from_min_size(Pos2::new(left, y), Vec2::new(width, row_h));
        let id = ui.make_persistent_id(("rail_slot", item.page));
        let resp = ui.interact(row, id, Sense::click());
        let is_active = item.page == st.active;
        let has_subs = !item.sub_tabs.is_empty();
        if resp.clicked() {
            action = Some(RailAction::Navigate(item.page));
        }
        let t = anim::anim_bool(
            &ctx,
            id.with("active"),
            is_active && !has_subs,
            0.16,
            anim::ease_out_cubic,
        );
        let hover = anim::anim_bool(
            &ctx,
            id.with("hover"),
            resp.hovered(),
            0.1,
            anim::ease_out_cubic,
        );
        row_fill(ui, row, t, hover, s);
        let col = if is_active {
            theme::INK
        } else {
            theme::lerp_color(theme::INK_MUTED, theme::INK_DIM, hover)
        };
        ui.painter().text(
            Pos2::new(row.left() + px(18.0), row.center().y),
            Align2::CENTER_CENTER,
            item.glyph.to_string(),
            theme::icon_font(px(16.0), is_active),
            if is_active {
                theme::INK
            } else {
                theme::INK_MUTED
            },
        );
        ui.painter().text(
            Pos2::new(row.left() + px(34.0), row.center().y),
            Align2::LEFT_CENTER,
            item.label,
            FontId::new(px(13.0), theme::family_medium()),
            col,
        );
        if has_subs {
            let open =
                anim::anim_bool(&ctx, id.with("open"), is_active, 0.18, anim::ease_out_cubic);
            ui.painter().text(
                Pos2::new(row.right() - px(12.0), row.center().y),
                Align2::CENTER_CENTER,
                if open > 0.5 {
                    theme::icon::CARET_DOWN
                } else {
                    CARET_RIGHT
                }
                .to_string(),
                theme::icon_font(px(10.0), true),
                theme::with_alpha(theme::INK_FAINT, (80.0 + 175.0 * hover.max(open)) as u8),
            );
            y += row_h + px(2.0);
            if open > 0.01 {
                let sub_h = px(28.0);
                let block_h = (sub_h + px(2.0)) * item.sub_tabs.len() as f32 * open;
                let guide_x = row.left() + px(18.0);
                ui.painter().line_segment(
                    [
                        Pos2::new(guide_x, y + px(2.0)),
                        Pos2::new(guide_x, y + block_h - px(4.0)),
                    ],
                    Stroke::new(1.0, theme::with_alpha(theme::LINE, (255.0 * open) as u8)),
                );
                for (ti, tab) in item.sub_tabs.iter().enumerate() {
                    let sub = Rect::from_min_size(
                        Pos2::new(left + px(28.0), y),
                        Vec2::new(width - px(28.0), sub_h),
                    );
                    let sid = id.with(("sub", ti));
                    let on = is_active && st.sub_tabs[i] == ti;
                    let sresp = ui.interact(sub, sid, Sense::click());
                    let st_on =
                        anim::anim_bool(&ctx, sid.with("a"), on, 0.16, anim::ease_out_cubic);
                    let sh = anim::anim_bool(
                        &ctx,
                        sid.with("h"),
                        sresp.hovered(),
                        0.1,
                        anim::ease_out_cubic,
                    );
                    let fade = open.clamp(0.0, 1.0);
                    if fade > 0.6 {
                        row_fill(ui, sub, st_on, sh, s);
                    }
                    let c = if on {
                        theme::INK
                    } else {
                        theme::lerp_color(theme::INK_MUTED, theme::INK_DIM, sh)
                    };
                    ui.painter().text(
                        Pos2::new(sub.left() + px(10.0), sub.center().y),
                        Align2::LEFT_CENTER,
                        *tab,
                        FontId::new(px(13.0), theme::family_medium()),
                        anim::with_opacity(c, fade),
                    );
                    if sresp.clicked() && fade > 0.6 {
                        action = Some(RailAction::SubTab(item.page, ti));
                    }
                    y += (sub_h + px(2.0)) * open;
                }
                y += px(4.0) * open;
            }
        } else {
            y += row_h + px(2.0);
        }
    }

    let acct = account_row(rect, s);
    ui.painter().line_segment(
        [
            Pos2::new(rect.left() + px(16.0), acct.top() - px(10.0)),
            Pos2::new(rect.right() - px(16.0), acct.top() - px(10.0)),
        ],
        Stroke::new(1.0, theme::LINE_SOFT),
    );
    let aid = ui.make_persistent_id("rail_avatar");
    let aresp = ui.interact(acct, aid, Sense::click());
    let ahover = anim::anim_bool(&ctx, aid, aresp.hovered(), 0.12, anim::ease_out_cubic);
    row_fill(ui, acct, 0.0, ahover, s);
    let avatar_r = px(11.0);
    let avatar_c = Pos2::new(acct.left() + px(8.0) + avatar_r, acct.center().y);
    ui.painter()
        .circle_filled(avatar_c, avatar_r, theme::with_alpha(theme::ACCENT, 70));
    ui.painter().circle_stroke(
        avatar_c,
        avatar_r,
        Stroke::new(1.0, theme::with_alpha(theme::ACCENT_HI, 120)),
    );
    ui.painter().text(
        avatar_c,
        Align2::CENTER_CENTER,
        st.avatar.to_string(),
        FontId::new(px(11.0), theme::family_semibold()),
        theme::INK,
    );
    let name = if st.account.is_empty() {
        "No account"
    } else {
        st.account
    };
    ui.painter().text(
        Pos2::new(avatar_c.x + avatar_r + px(10.0), acct.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::new(px(13.0), theme::family_medium()),
        if st.account.is_empty() {
            theme::INK_MUTED
        } else {
            theme::INK_DIM
        },
    );
    ui.painter().text(
        Pos2::new(acct.right() - px(10.0), acct.center().y),
        Align2::RIGHT_CENTER,
        theme::icon::CARET_DOWN.to_string(),
        theme::icon_font(px(11.0), false),
        theme::INK_FAINT,
    );
    if aresp.clicked() {
        action = Some(RailAction::Avatar);
    }

    action
}

pub const CARET_RIGHT: char = '\u{e13a}';

pub fn kbd(ui: &egui::Ui, right: Pos2, text: &str, s: f32) -> Rect {
    let px = |v: f32| theme::px(v, s);
    let font = FontId::new(px(11.0), theme::family_medium());
    let w = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), theme::INK_MUTED)
        .size()
        .x;
    let r = Rect::from_min_max(
        Pos2::new(right.x - w - px(12.0), right.y - px(9.0)),
        Pos2::new(right.x, right.y + px(9.0)),
    );
    ui.painter()
        .rect_filled(r, theme::corner(px(4.0)), theme::FIELD_HI);
    ui.painter().text(
        r.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        theme::INK_MUTED,
    );
    r
}
