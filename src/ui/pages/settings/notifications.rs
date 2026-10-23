use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use crate::app::App;
use crate::features::notify;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::theme;

pub struct NotificationsState {
    webhook: String,
    loaded: bool,
    had_saved: bool,
    tx: Sender<Result<(), String>>,
    rx: Receiver<Result<(), String>>,
}

impl Default for NotificationsState {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            webhook: String::new(),
            loaded: false,
            had_saved: false,
            tx,
            rx,
        }
    }
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
    if !app.notifications_ui.loaded {
        app.notifications_ui.webhook = crate::secrets::get_password(notify::WEBHOOK_KEY)
            .ok()
            .flatten()
            .unwrap_or_default();
        app.notifications_ui.had_saved = !app.notifications_ui.webhook.is_empty();
        app.notifications_ui.loaded = true;
    }
    while let Ok(res) = app.notifications_ui.rx.try_recv() {
        match res {
            Ok(()) => app
                .toasts
                .push(kit::ToastLevel::Success, "Test message sent"),
            Err(e) => app
                .toasts
                .push(kit::ToastLevel::Error, format!("Test failed: {e}")),
        }
    }
    let mut cfg = settings();

    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left(), rect.top()),
        "Build notifications",
        s,
    );
    let mut y = rect.top() + px(32.0);
    toggle_row(
        ui,
        rect.left(),
        &mut y,
        "Show a desktop toast when a build finishes",
        &mut cfg.notify_toast,
        &save,
        s,
    );
    toggle_row(
        ui,
        rect.left() + px(24.0),
        &mut y,
        "Notify on success",
        &mut cfg.notify_success,
        &save,
        s,
    );
    toggle_row(
        ui,
        rect.left() + px(24.0),
        &mut y,
        "Notify on failure or cancel",
        &mut cfg.notify_failure,
        &save,
        s,
    );

    y += px(16.0);
    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left(), y),
        "Discord webhook",
        s,
    );
    y += px(32.0);
    toggle_row(
        ui,
        rect.left(),
        &mut y,
        "Post build status to a Discord channel",
        &mut cfg.discord_enabled,
        &save,
        s,
    );

    ui.painter().text(
        Pos2::new(rect.left(), y),
        Align2::LEFT_TOP,
        "Webhook URL",
        FontId::new(px(11.5), theme::family_label()),
        theme::INK_MUTED,
    );
    let field = Rect::from_min_size(
        Pos2::new(rect.left(), y + px(18.0)),
        Vec2::new(px(420.0), px(36.0)),
    );
    Input::new(&mut app.notifications_ui.webhook)
        .hint("https://discord.com/api/webhooks/…")
        .mask(true)
        .show(ui, field, s);
    let save_btn = Rect::from_min_size(
        Pos2::new(field.right() + px(10.0), field.top()),
        Vec2::new(px(90.0), px(36.0)),
    );
    if Button::new("Save", ButtonKind::Primary)
        .show(ui, save_btn, s)
        .clicked()
    {
        store_webhook(app);
    }
    let clear_btn = Rect::from_min_size(
        Pos2::new(save_btn.right() + px(8.0), field.top()),
        Vec2::new(px(90.0), px(36.0)),
    );
    if Button::new("Clear", ButtonKind::Secondary)
        .show(ui, clear_btn, s)
        .clicked()
    {
        let _ = crate::secrets::delete_password(notify::WEBHOOK_KEY);
        app.notifications_ui.webhook.clear();
        app.notifications_ui.had_saved = false;
        app.toasts.push(kit::ToastLevel::Success, "Webhook cleared");
    }

    let status_y = field.bottom() + px(10.0);
    let status = if app.notifications_ui.had_saved {
        "A webhook is saved in your OS keyring."
    } else {
        "No webhook saved. It is stored in the keyring, never in plain settings."
    };
    ui.painter().text(
        Pos2::new(rect.left(), status_y),
        Align2::LEFT_TOP,
        status,
        FontId::new(px(11.5), theme::family_medium()),
        theme::INK_MUTED,
    );

    let test = Rect::from_min_size(
        Pos2::new(rect.left(), status_y + px(24.0)),
        Vec2::new(px(180.0), px(36.0)),
    );
    if Button::new("Send test message", ButtonKind::Secondary)
        .show(ui, test, s)
        .clicked()
    {
        send_test(app, &ctx);
    }
}

fn store_webhook(app: &mut App) {
    let value = app.notifications_ui.webhook.trim().to_string();
    if value.is_empty() {
        app.toasts
            .push(kit::ToastLevel::Warning, "Enter a webhook URL first");
        return;
    }
    match crate::secrets::set_password(notify::WEBHOOK_KEY, &value) {
        Ok(()) => {
            app.notifications_ui.had_saved = true;
            app.toasts
                .push(kit::ToastLevel::Success, "Webhook saved to keyring");
        }
        Err(e) => app
            .toasts
            .push(kit::ToastLevel::Error, format!("Could not save: {e}")),
    }
}

fn send_test(app: &mut App, ctx: &egui::Context) {
    let webhook = app.notifications_ui.webhook.trim().to_string();
    let tx = app.notifications_ui.tx.clone();
    let ctx = ctx.clone();
    app.toasts
        .push(kit::ToastLevel::Success, "Sending test message…");
    std::thread::spawn(move || {
        let info = notify::test_message();
        let result = notify::post_discord(&webhook, &info);
        let _ = tx.send(result);
        ctx.request_repaint();
    });
}

fn toggle_row(
    ui: &mut egui::Ui,
    x: f32,
    y: &mut f32,
    label: &str,
    value: &mut bool,
    save: &dyn Fn(&crate::config::Settings),
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let tog = Rect::from_min_size(Pos2::new(x, *y), Vec2::new(px(44.0), px(24.0)));
    let mut on = *value;
    if kit::toggle(ui, tog, &mut on, s).changed() {
        *value = on;
        let mut cfg = settings();
        apply(&mut cfg, label, on);
        save(&cfg);
    }
    ui.painter().text(
        Pos2::new(tog.right() + px(12.0), tog.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_DIM,
    );
    *y += px(38.0);
}

fn apply(cfg: &mut crate::config::Settings, label: &str, on: bool) {
    match label {
        "Show a desktop toast when a build finishes" => cfg.notify_toast = on,
        "Notify on success" => cfg.notify_success = on,
        "Notify on failure or cancel" => cfg.notify_failure = on,
        "Post build status to a Discord channel" => cfg.discord_enabled = on,
        _ => {}
    }
}
