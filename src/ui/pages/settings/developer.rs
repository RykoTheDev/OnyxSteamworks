use eframe::egui::{self, Align2, FontId, Key, Pos2, Rect, Vec2};

use crate::app::App;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::theme;

#[derive(Default)]
pub struct DeveloperState {
    override_path: String,
    loaded: bool,
    confirm_reset: bool,
}

impl DeveloperState {
    pub fn reload(&mut self) {
        self.loaded = false;
    }
}

fn cfg() -> crate::config::Settings {
    crate::config::load_settings().unwrap_or_default()
}

fn save(c: &crate::config::Settings) {
    if let Err(e) = crate::config::save_settings(c) {
        eprintln!("could not save settings: {e}");
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    if !app.developer_ui.loaded {
        app.developer_ui.override_path = cfg().steamcmd_override;
        app.developer_ui.loaded = true;
    }

    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left(), rect.top()),
        "Developer",
        s,
    );
    let card = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + px(26.0)),
        Vec2::new(rect.width().min(px(560.0)), px(170.0)),
    );
    kit::bordered(
        &ui.painter().clone(),
        card,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    let x = card.left() + px(20.0);
    let w = card.width() - px(40.0);

    let mut c = cfg();
    let tog = Rect::from_min_size(
        Pos2::new(x, card.top() + px(18.0)),
        Vec2::new(px(44.0), px(24.0)),
    );
    let mut fake = c.use_fake_steamcmd;
    if kit::toggle(ui, tog, &mut fake, s).changed() {
        c.use_fake_steamcmd = fake;
        save(&c);
    }
    ui.painter().text(
        Pos2::new(tog.right() + px(12.0), tog.center().y),
        Align2::LEFT_CENTER,
        "Use the fake SteamCMD (developer testing)",
        FontId::new(px(12.5), theme::family_semibold()),
        theme::INK,
    );

    ui.painter().text(
        Pos2::new(x, card.top() + px(58.0)),
        Align2::LEFT_TOP,
        "SteamCMD path override (blank = per-project path)",
        FontId::new(px(11.5), theme::family_label()),
        theme::INK_MUTED,
    );
    let field = Rect::from_min_size(Pos2::new(x, card.top() + px(78.0)), Vec2::new(w, px(34.0)));
    Input::new(&mut app.developer_ui.override_path)
        .hint("C:\\tools\\steamcmd\\steamcmd.exe")
        .show(ui, field, s);
    if ui.input(|i| i.key_pressed(Key::Enter) || i.pointer.any_released())
        && app.developer_ui.override_path != c.steamcmd_override
    {
        c.steamcmd_override = app.developer_ui.override_path.clone();
        save(&c);
    }

    let open = Rect::from_min_size(
        Pos2::new(x, card.bottom() - px(44.0)),
        Vec2::new(px(200.0), px(34.0)),
    );
    if Button::new("Open app data folder", ButtonKind::Secondary)
        .icon(theme::icon::FOLDER)
        .show(ui, open, s)
        .clicked()
        && let Ok(dir) = crate::config::data_dir()
    {
        std::thread::spawn(move || {
            let _ = open::that(dir);
        });
    }
    let reset = Rect::from_min_size(
        Pos2::new(card.right() - px(160.0), card.bottom() - px(44.0)),
        Vec2::new(px(160.0), px(34.0)),
    );
    if Button::new("Reset settings", ButtonKind::Danger)
        .icon(theme::icon::TRASH)
        .show(ui, reset, s)
        .clicked()
    {
        app.developer_ui.confirm_reset = true;
    }

    reset_modal(app, ui, s);
}

fn reset_modal(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    if !app.developer_ui.confirm_reset {
        return;
    }
    let id = ui.make_persistent_id("reset_settings_modal");
    let mut open = true;
    let mut confirm = false;
    let mut cancel = false;
    kit::modal(
        ui,
        id,
        &mut open,
        Vec2::new(px(420.0), px(190.0)),
        s,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(26.0);
            kit::label(
                &ui.painter().clone(),
                Pos2::new(x, d.top() + px(26.0)),
                "Reset settings",
                s,
            );
            ui.painter().text(
                Pos2::new(x, d.top() + px(58.0)),
                Align2::LEFT_TOP,
                "This restores all settings to defaults. Your projects are kept.",
                FontId::new(px(12.5), theme::family_medium()),
                theme::INK_DIM,
            );
            let bw = px(150.0);
            let by = d.bottom() - px(26.0) - px(40.0);
            if Button::new("Cancel", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, by), Vec2::new(bw, px(40.0))),
                    s,
                )
                .clicked()
            {
                cancel = true;
            }
            if Button::new("Reset", ButtonKind::Danger)
                .icon(theme::icon::TRASH)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - px(26.0) - bw, by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked()
            {
                confirm = true;
            }
            confirm || cancel
        },
    );
    if confirm {
        save(&crate::config::Settings::default());
        theme::set_ui_scale(1.0);
        crate::ui::anim::set_reduce_motion(false);
        app.developer_ui.reload();
        app.advanced_ui.reload();
        app.developer_ui.confirm_reset = false;
        app.toasts
            .push(kit::ToastLevel::Success, "Settings reset to defaults");
    } else if cancel || !open {
        app.developer_ui.confirm_reset = false;
    }
}
