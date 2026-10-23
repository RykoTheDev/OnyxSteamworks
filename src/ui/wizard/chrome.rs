use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Vec2};

use super::{Intent, STEPS, View, steps};
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

pub(super) fn chrome(ui: &mut egui::Ui, d: Rect, view: &View, intent: &mut Intent, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let x = d.left() + px(28.0);
    let painter = ui.painter().clone();
    kit::label(
        &painter,
        Pos2::new(x, d.top() + px(26.0)),
        "Set up SteamPipe",
        s,
    );
    painter.text(
        Pos2::new(x, d.top() + px(48.0)),
        Align2::LEFT_TOP,
        steps::title(view.step),
        FontId::new(px(19.0), theme::family_semibold()),
        theme::INK,
    );
    painter.text(
        Pos2::new(d.right() - px(56.0), d.top() + px(28.0)),
        Align2::RIGHT_CENTER,
        format!("STEP {} / {}", view.step + 1, STEPS),
        FontId::new(px(11.0), theme::family_label()),
        theme::INK_MUTED,
    );
    let close = Rect::from_min_size(
        Pos2::new(d.right() - px(44.0), d.top() + px(14.0)),
        Vec2::splat(px(28.0)),
    );
    if kit::icon_button(ui, close, theme::icon::X, s).clicked() {
        intent.close = true;
    }
    let seg = Rect::from_min_size(
        Pos2::new(x, d.top() + px(90.0)),
        Vec2::new(d.width() - px(56.0), px(5.0)),
    );
    kit::segment_progress(ui, seg, STEPS, view.step + 1, s);
}

pub(super) fn footer(ui: &mut egui::Ui, d: Rect, view: &View, intent: &mut Intent, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let bh = px(42.0);
    let y = d.bottom() - px(26.0) - bh;
    let last = view.step + 1 == STEPS;

    if view.step > 0
        && Button::new("Back", ButtonKind::Secondary)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(d.left() + px(28.0), y), Vec2::new(px(120.0), bh)),
                s,
            )
            .clicked()
    {
        intent.back = true;
    }

    let right = d.right() - px(28.0);
    if last {
        if Button::new("Close", ButtonKind::Secondary)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(right - px(332.0), y), Vec2::new(px(110.0), bh)),
                s,
            )
            .clicked()
        {
            intent.close = true;
        }
        if Button::new("Start a preview build", ButtonKind::Primary)
            .icon(theme::icon::ROCKET)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(right - px(210.0), y), Vec2::new(px(210.0), bh)),
                s,
            )
            .clicked()
        {
            intent.preview_build = true;
        }
    } else {
        if Button::new("Skip", ButtonKind::Ghost)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(right - px(224.0), y), Vec2::new(px(96.0), bh)),
                s,
            )
            .clicked()
        {
            intent.skip = true;
        }
        if Button::new("Next", ButtonKind::Primary)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(right - px(120.0), y), Vec2::new(px(120.0), bh)),
                s,
            )
            .clicked()
        {
            intent.next = true;
        }
    }
}

pub(super) fn para(
    ui: &egui::Ui,
    pos: Pos2,
    width: f32,
    text: &str,
    color: Color32,
    s: f32,
) -> f32 {
    let font = FontId::new(theme::px(13.0, s), theme::family_medium());
    let galley = ui.painter().layout(text.to_owned(), font, color, width);
    let h = galley.size().y;
    ui.painter().galley(pos, galley, color);
    h
}

pub(super) fn mb(n: u64) -> String {
    format!("{:.1} MB", n as f64 / 1_048_576.0)
}
