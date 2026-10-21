use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Vec2};

use crate::app::{App, Page};
use crate::ui::kit;
use crate::ui::{nav, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, rail: Rect, s: f32) {
    if !app.account_menu {
        return;
    }
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let accounts = crate::config::load_settings()
        .map(|c| c.accounts)
        .unwrap_or_default();
    let sel = app.build_ui.project_sel;
    let current = app
        .projects
        .get(sel)
        .map(|p| p.default_account.clone())
        .unwrap_or_default();
    let rows = accounts.len() + 1;
    let w = px(200.0);
    let h = px(18.0) + rows as f32 * px(34.0);
    let anchor = Pos2::new(rail.right() + px(8.0), rail.bottom() - px(44.0) - h);
    let mut pick = None;
    let mut manage = false;
    let area = egui::Area::new(ui.make_persistent_id("account_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor)
        .show(&ctx, |ui| {
            let rect = Rect::from_min_size(anchor, Vec2::new(w, h));
            kit::bordered(
                ui.painter(),
                rect,
                theme::SURFACE_2,
                theme::LINE,
                px(theme::RADIUS_POPUP),
            );
            kit::label(
                &ui.painter().clone(),
                Pos2::new(rect.left() + px(14.0), rect.top() + px(10.0)),
                "ACCOUNTS",
                s,
            );
            let mut y = rect.top() + px(32.0);
            for name in &accounts {
                let r = Rect::from_min_size(
                    Pos2::new(rect.left() + px(6.0), y),
                    Vec2::new(w - px(12.0), px(30.0)),
                );
                let resp = ui.interact(r, ui.make_persistent_id(("acctrow", name)), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(
                        r,
                        theme::corner(px(theme::RADIUS_FIELD)),
                        theme::FIELD,
                    );
                }
                let on = name.eq_ignore_ascii_case(&current) && !current.is_empty();
                ui.painter().text(
                    Pos2::new(r.left() + px(10.0), r.center().y),
                    Align2::LEFT_CENTER,
                    name,
                    FontId::new(px(12.5), theme::family_medium()),
                    theme::INK,
                );
                if on {
                    ui.painter().text(
                        Pos2::new(r.right() - px(10.0), r.center().y),
                        Align2::RIGHT_CENTER,
                        theme::icon::CHECK.to_string(),
                        theme::icon_font(px(13.0), false),
                        theme::SUCCESS,
                    );
                }
                if resp.clicked() {
                    pick = Some(name.clone());
                }
                y += px(34.0);
            }
            let mr = Rect::from_min_size(
                Pos2::new(rect.left() + px(6.0), y),
                Vec2::new(w - px(12.0), px(30.0)),
            );
            let mresp = ui.interact(mr, ui.make_persistent_id("acct_manage"), Sense::click());
            if mresp.hovered() {
                ui.painter()
                    .rect_filled(mr, theme::corner(px(theme::RADIUS_FIELD)), theme::FIELD);
            }
            ui.painter().text(
                Pos2::new(mr.left() + px(10.0), mr.center().y),
                Align2::LEFT_CENTER,
                "Manage accounts \u{2192}",
                FontId::new(px(12.0), theme::family_medium()),
                theme::INK_DIM,
            );
            if mresp.clicked() {
                manage = true;
            }
        });
    if area.response.clicked_elsewhere() {
        app.account_menu = false;
    }
    if let Some(name) = pick {
        set_account(app, sel, &name);
        app.account_menu = false;
    }
    if manage {
        app.sub_tabs[nav::index(Page::Settings)] = 1;
        app.nav_request = Some(Page::Settings);
        app.account_menu = false;
    }
}

fn set_account(app: &mut App, sel: usize, account: &str) {
    if let Some(p) = app.projects.get_mut(sel) {
        p.steam_username = account.to_string();
        p.default_account = account.to_string();
        let project = p.clone();
        let _ = crate::config::save_project(&project);
        app.toasts
            .push(kit::ToastLevel::Success, format!("Switched to {account}"));
    }
}
