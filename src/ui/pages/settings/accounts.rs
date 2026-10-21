use eframe::egui::{self, Align2, FontId, Pos2, Rect, UiBuilder, Vec2};

use crate::app::{App, Page};
use crate::features::accounts;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::theme;

#[derive(Default)]
pub struct AccountsState {
    pub input: String,
}

fn settings() -> crate::config::Settings {
    crate::config::load_settings().unwrap_or_default()
}

fn save(cfg: &crate::config::Settings) {
    if let Err(e) = crate::config::save_settings(cfg) {
        eprintln!("could not save settings: {e}");
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let mut cfg = settings();

    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left(), rect.top()),
        "STEAM ACCOUNTS",
        s,
    );
    ui.painter().text(
        Pos2::new(rect.left(), rect.top() + px(24.0)),
        Align2::LEFT_TOP,
        "Store build-account usernames here. Passwords are never saved unless you opt in to the keyring.",
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK_MUTED,
    );

    let field = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + px(48.0)),
        Vec2::new(px(300.0), px(36.0)),
    );
    Input::new(&mut app.accounts_ui.input)
        .hint("Add a Steam username")
        .show(ui, field, s);
    let add = Rect::from_min_size(
        Pos2::new(field.right() + px(10.0), field.top()),
        Vec2::new(px(110.0), px(36.0)),
    );
    if Button::new("ADD", ButtonKind::Primary)
        .icon(theme::icon::PLUS)
        .show(ui, add, s)
        .clicked()
    {
        let name = app.accounts_ui.input.trim().to_string();
        if accounts::add(&mut cfg.accounts, &name) {
            save(&cfg);
            app.accounts_ui.input.clear();
            app.toasts.push(kit::ToastLevel::Success, "Account added");
        }
    }

    let steamcmd = app
        .projects
        .get(app.build_ui.project_sel)
        .map(|p| p.steamcmd_path.clone());
    let default_acct = app
        .projects
        .get(app.build_ui.project_sel)
        .map(|p| p.default_account.clone())
        .unwrap_or_default();
    let list = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + px(100.0)),
        Pos2::new(rect.right(), rect.bottom() - px(56.0)),
    );
    if cfg.accounts.is_empty() {
        kit::empty_state(
            ui,
            list,
            theme::icon::STACK,
            "No accounts yet",
            "Add a username above to switch between build accounts.",
            s,
        );
    } else {
        let accounts_list = cfg.accounts.clone();
        let mut remove = None;
        let mut authorize = None;
        let mut cu = ui.new_child(UiBuilder::new().max_rect(list));
        cu.set_clip_rect(list);
        egui::ScrollArea::vertical()
            .id_salt("accounts_list")
            .auto_shrink([false, false])
            .show(&mut cu, |ui| {
                ui.set_width(list.width() - px(14.0));
                for name in &accounts_list {
                    let (r, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), px(52.0)),
                        egui::Sense::hover(),
                    );
                    kit::bordered(
                        ui.painter(),
                        r,
                        theme::SURFACE_2,
                        theme::LINE,
                        px(theme::RADIUS_CARD),
                    );
                    let authed = steamcmd
                        .as_ref()
                        .map(|p| accounts::is_authorized(p, name))
                        .unwrap_or(false);
                    let dot = if authed {
                        theme::SUCCESS
                    } else {
                        theme::WARNING
                    };
                    ui.painter().circle_filled(
                        Pos2::new(r.left() + px(16.0), r.center().y),
                        px(4.0),
                        dot,
                    );
                    let mut title = name.clone();
                    if name.eq_ignore_ascii_case(&default_acct) && !default_acct.is_empty() {
                        title.push_str("  (default)");
                    }
                    ui.painter().text(
                        Pos2::new(r.left() + px(28.0), r.center().y - px(8.0)),
                        Align2::LEFT_CENTER,
                        title,
                        FontId::new(px(13.5), theme::family_semibold()),
                        theme::INK,
                    );
                    let status = if authed {
                        "Authorized cached login"
                    } else {
                        "No cached login"
                    };
                    ui.painter().text(
                        Pos2::new(r.left() + px(28.0), r.center().y + px(9.0)),
                        Align2::LEFT_CENTER,
                        status,
                        FontId::new(px(11.0), theme::family_medium()),
                        theme::INK_MUTED,
                    );

                    let bw = px(30.0);
                    let dz = Rect::from_min_size(
                        Pos2::new(r.right() - px(10.0) - bw, r.center().y - bw / 2.0),
                        Vec2::splat(bw),
                    );
                    if kit::icon_button(ui, dz, theme::icon::TRASH, s).clicked() {
                        remove = Some(name.clone());
                    }
                    if !authed {
                        let az = Rect::from_min_size(
                            Pos2::new(dz.left() - px(10.0) - px(120.0), r.center().y - px(16.0)),
                            Vec2::new(px(120.0), px(32.0)),
                        );
                        let can = steamcmd.is_some();
                        if Button::new("AUTHORIZE", ButtonKind::Secondary)
                            .enabled(can)
                            .disabled_reason("Select a project first")
                            .show(ui, az, s)
                            .clicked()
                        {
                            authorize = Some(name.clone());
                        }
                    }
                    ui.add_space(px(10.0));
                }
            });
        if let Some(name) = remove {
            accounts::remove(&mut cfg.accounts, &name);
            save(&cfg);
        }
        if let Some(name) = authorize {
            start_authorize(app, &name, &ctx);
        }
    }

    remember_toggle(ui, rect, &mut cfg, s);
}

fn remember_toggle(ui: &mut egui::Ui, rect: Rect, cfg: &mut crate::config::Settings, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let y = rect.bottom() - px(36.0);
    let tog = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(px(44.0), px(24.0)));
    let mut remember = cfg.remember_password;
    if kit::toggle(ui, tog, &mut remember, s).changed() {
        cfg.remember_password = remember;
        save(cfg);
    }
    ui.painter().text(
        Pos2::new(tog.right() + px(12.0), tog.center().y),
        Align2::LEFT_CENTER,
        "Remember passwords in the OS keyring (optional; off by default)",
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK_DIM,
    );
}

fn start_authorize(app: &mut App, account: &str, ctx: &egui::Context) {
    let sel = app.build_ui.project_sel;
    let Some(mut project) = app.projects.get(sel).cloned() else {
        return;
    };
    project.steam_username = account.to_string();
    project.default_account = account.to_string();
    if let Some(p) = app.projects.get_mut(sel) {
        p.steam_username = account.to_string();
        p.default_account = account.to_string();
    }
    let _ = crate::config::save_project(&project);
    match app.build_ui.authorize(&project, ctx) {
        Ok(()) => {
            app.nav_request = Some(Page::Build);
            app.toasts
                .push(kit::ToastLevel::Success, format!("Authorizing {account}"));
        }
        Err(e) => {
            app.build_ui
                .push_log(format!("Authorize could not start: {e}"), true);
            app.toasts
                .push(kit::ToastLevel::Error, "Authorize failed to start");
        }
    }
}
