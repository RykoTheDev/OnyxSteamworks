use eframe::egui::{self, FontId, Key, Pos2, Rect, Vec2};

use crate::app::App;
use crate::secrets;
use crate::steamcmd::PromptKind;
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

pub(super) fn show(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let Some(kind) = app.build_ui.pending_prompt else {
        return;
    };
    let px = |v: f32| theme::px(v, s);
    let mask = matches!(kind, PromptKind::Password);
    let title = match kind {
        PromptKind::Password => "Steam password",
        PromptKind::SteamGuard => "Steam guard code",
    };
    let hint = match kind {
        PromptKind::Password => "Password",
        PromptKind::SteamGuard => "Code from your Steam app or email",
    };

    let id = ui.make_persistent_id("auth_modal");
    let mut open = true;
    let mut submit = false;
    let mut cancel = false;
    let input = &mut app.build_ui.prompt_input;

    kit::modal_with(
        ui,
        id,
        &mut open,
        Vec2::new(px(440.0), px(258.0)),
        s,
        false,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(26.0);
            kit::label(
                &ui.painter().clone(),
                Pos2::new(x, d.top() + px(26.0)),
                title,
                s,
            );

            let note_font = FontId::new(px(11.5), theme::family_medium());
            let galley = ui.painter().layout(
                secrets::BUILD_ACCOUNT_NOTE.to_string(),
                note_font,
                theme::INK_MUTED,
                d.width() - px(52.0),
            );
            ui.painter()
                .galley(Pos2::new(x, d.top() + px(52.0)), galley, theme::INK_MUTED);

            let field = Rect::from_min_size(
                Pos2::new(x, d.bottom() - px(118.0)),
                Vec2::new(d.width() - px(52.0), px(40.0)),
            );
            let resp = Input::new(input).hint(hint).mask(mask).show(ui, field, s);
            let entered = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));

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
            let send = Button::new("Submit", ButtonKind::Primary)
                .icon(theme::icon::CHECK)
                .enabled(!input.is_empty())
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - px(26.0) - bw, by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked();
            if (send || entered) && !input.is_empty() {
                submit = true;
            }
            submit || cancel
        },
    );

    if submit {
        app.build_ui.respond();
    } else if cancel {
        app.build_ui.cancel();
    }
}
