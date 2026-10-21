use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use crate::app::App;
use crate::ui::layout::Layout;
use crate::ui::{anim, kit, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let s = layout.scale;
    let px = |v: f32| theme::px(v, s);
    let content = layout.content;

    kit::hero(&ui.painter().clone(), content, "STEAMPIPE", s);

    let (op, dy) = anim::stagger_in(
        ui.ctx(),
        ui.make_persistent_id(("home", app.home_gen)),
        0,
        0.0,
    );
    let body = Rect::from_min_max(
        Pos2::new(content.left(), content.top() + px(104.0) + dy * s),
        content.max,
    );
    let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(body));
    cu.set_opacity(op);
    let ui = &mut cu;

    ui.painter().text(
        body.left_top(),
        Align2::LEFT_TOP,
        "Ship builds to Steam without the command line.",
        FontId::new(px(17.0), theme::family_semibold()),
        theme::INK,
    );
    ui.painter().text(
        Pos2::new(body.left(), body.top() + px(28.0)),
        Align2::LEFT_TOP,
        "Configure a project, generate the VDFs, and run SteamCMD from one place.",
        FontId::new(px(13.0), theme::family_medium()),
        theme::INK_MUTED,
    );

    let cards_top = body.top() + px(76.0);
    let gap = px(16.0);
    let card_w = ((content.width() - gap * 2.0) / 3.0).min(px(240.0));
    let card_h = px(104.0);
    let count = app.projects.len();
    let cards = [
        ("PROJECTS", count.to_string()),
        (
            "DEPOTS",
            app.projects
                .iter()
                .map(|p| p.depots.len())
                .sum::<usize>()
                .to_string(),
        ),
        ("LAST BUILD", "--".to_string()),
    ];
    for (i, (label, value)) in cards.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(content.left() + i as f32 * (card_w + gap), cards_top),
            Vec2::new(card_w, card_h),
        );
        kit::bordered(
            ui.painter(),
            r,
            theme::SURFACE_2,
            theme::LINE,
            px(theme::RADIUS_CARD),
        );
        ui.painter().text(
            Pos2::new(r.left() + px(18.0), r.top() + px(18.0)),
            Align2::LEFT_TOP,
            label,
            FontId::new(px(12.0), theme::family_label()),
            theme::INK_MUTED,
        );
        ui.painter().text(
            Pos2::new(r.left() + px(18.0), r.bottom() - px(16.0)),
            Align2::LEFT_BOTTOM,
            value,
            FontId::new(px(34.0), theme::family_semibold()),
            theme::INK,
        );
    }

    ui.painter().text(
        Pos2::new(content.left(), cards_top + card_h + px(26.0)),
        Align2::LEFT_TOP,
        "Open Projects from the rail to get started.",
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_MUTED,
    );
}
