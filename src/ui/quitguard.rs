use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2, ViewportCommand};

use crate::app::App;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let ctx = ui.ctx().clone();

    if app.quitting {
        ctx.send_viewport_cmd(ViewportCommand::Close);
        return;
    }

    if ctx.input(|i| i.viewport().close_requested()) {
        if app.build_ui.running {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            app.confirm_quit = true;
        } else {
            return;
        }
    }

    if !app.confirm_quit {
        return;
    }
    if !app.build_ui.running {
        app.confirm_quit = false;
        return;
    }

    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id("confirm_quit");
    let mut open = true;
    let mut quit = false;
    let mut cancel = false;
    kit::modal_with(
        ui,
        id,
        &mut open,
        Vec2::new(px(468.0), px(224.0)),
        s,
        false,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(28.0);
            let painter = ui.painter().clone();
            kit::label(
                &painter,
                Pos2::new(x, d.top() + px(26.0)),
                "Build in progress",
                s,
            );
            painter.text(
                Pos2::new(x, d.top() + px(48.0)),
                Align2::LEFT_TOP,
                "Quit and cancel the running build?",
                FontId::new(px(16.0), theme::family_semibold()),
                theme::INK,
            );
            painter.text(
                Pos2::new(x, d.top() + px(80.0)),
                Align2::LEFT_TOP,
                "SteamCMD will be stopped and this build will not finish.",
                FontId::new(px(12.5), theme::family_medium()),
                theme::INK_MUTED,
            );
            let by = d.bottom() - px(26.0) - px(42.0);
            if Button::new("Keep building", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, by), Vec2::new(px(170.0), px(42.0))),
                    s,
                )
                .clicked()
            {
                cancel = true;
            }
            if Button::new("Quit & cancel", ButtonKind::Danger)
                .icon(theme::icon::WARNING)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - px(28.0) - px(170.0), by),
                        Vec2::new(px(170.0), px(42.0)),
                    ),
                    s,
                )
                .clicked()
            {
                quit = true;
            }
            false
        },
    );

    if quit {
        app.build_ui.cancel();
        app.confirm_quit = false;
        app.quitting = true;
    } else if cancel || !open {
        app.confirm_quit = false;
    }
}
