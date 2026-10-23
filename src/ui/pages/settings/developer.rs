use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use crate::app::App;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

#[derive(Default)]
pub struct DeveloperState {
    confirm_reset: bool,
}

impl DeveloperState {
    pub fn reload(&mut self) {}
}

fn save(c: &crate::config::Settings) {
    if let Err(e) = crate::config::save_settings(c) {
        eprintln!("could not save settings: {e}");
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left(), rect.top()),
        "Developer",
        s,
    );
    let card = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + px(26.0)),
        Vec2::new(rect.width().min(px(560.0)), px(74.0)),
    );
    kit::bordered(
        &ui.painter().clone(),
        card,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    let x = card.left() + px(20.0);
    let open = Rect::from_min_size(
        Pos2::new(x, card.center().y - px(17.0)),
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
        Pos2::new(card.right() - px(180.0), card.center().y - px(17.0)),
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
