use eframe::egui::{self, Align2, FontId, Pos2, Rect, UiBuilder, Vec2};

use crate::app::App;
use crate::ui::bento::{
    chip, fan_cards, notch_button, panel, prompt_bar, search_pill, segment_progress, stack_cards,
};
use crate::ui::layout::Layout;
use crate::ui::{anim, theme};

const BRANCHES: [&str; 4] = ["DEV", "BETA", "STAGING", "LIVE"];

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let s = layout.scale;
    let px = |v: f32| theme::px(v, s);
    let group = ui.make_persistent_id(("home_stagger", app.home_gen));

    let name = app
        .projects
        .first()
        .map(|p| p.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "UNTITLED PROJECT".to_string());

    let (op0, off0) = entrance(ui.ctx(), group, 0, s);
    {
        let hero = layout.hero.translate(off0);
        let bite = layout.hero_bite.translate(off0);
        let prompt = layout.prompt_bar.translate(off0);
        let notch = layout.notch + off0;
        let mut cu = ui.new_child(UiBuilder::new().max_rect(hero));
        cu.set_opacity(op0);
        panel::panel(
            &cu.painter().clone(),
            hero,
            theme::PANEL_ALT,
            px(theme::RADIUS_PANEL),
        );
        cu.painter().text(
            Pos2::new(hero.left() + px(44.0), hero.top() + px(44.0)),
            Align2::LEFT_TOP,
            "KEY ART",
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
        cu.painter().text(
            Pos2::new(hero.left() + px(44.0), prompt.top() - px(28.0)),
            Align2::LEFT_BOTTOM,
            name.to_uppercase(),
            FontId::new(px(22.0), theme::family_semibold()),
            theme::INK,
        );
        panel::cutout(&cu.painter().clone(), bite, px(theme::RADIUS_PANEL));
        prompt_bar::prompt_bar(&mut cu, prompt, &mut app.build_desc, s);
        notch_button::notch_button(&mut cu, notch, s);
    }

    let (op2, off2) = entrance(ui.ctx(), group, 2, s);
    {
        let choose = layout.choose.translate(off2);
        let mut cu = ui.new_child(UiBuilder::new().max_rect(choose));
        cu.set_opacity(op2);
        panel::panel(
            &cu.painter().clone(),
            choose,
            theme::PANEL,
            px(theme::RADIUS_PANEL_MIN),
        );
        cu.painter().text(
            Pos2::new(choose.left() + px(24.0), choose.top() + px(24.0)),
            Align2::LEFT_TOP,
            "CHOOSE BRANCH",
            FontId::new(px(14.0), theme::family_semibold()),
            theme::INK,
        );
        let grid = Rect::from_min_size(
            Pos2::new(choose.left() + px(24.0), choose.top() + px(60.0)),
            Vec2::new(choose.width() - px(48.0), px(96.0)),
        );
        chip::chip_grid(&mut cu, grid, &BRANCHES, &mut app.branch_sel, false, s);
        let active = app.branch_sel.first().copied().unwrap_or(0).min(4);
        segment_progress::segment_progress(
            &mut cu,
            Pos2::new(choose.left() + px(24.0), choose.bottom() - px(24.0)),
            5,
            active,
            s,
        );
    }

    if layout.show_right {
        let (op4, off4) = entrance(ui.ctx(), group, 4, s);
        let mut cu = ui.new_child(UiBuilder::new().max_rect(layout.search.translate(off4)));
        cu.set_opacity(op4);
        search_pill::search_pill(&mut cu, layout.search.translate(off4), &mut app.search, s);

        let (op1, off1) = entrance(ui.ctx(), group, 1, s);
        let stack = layout.stack.translate(off1);
        let mut cs = ui.new_child(UiBuilder::new().max_rect(stack));
        cs.set_opacity(op1);
        let cards = [
            stack_cards::StackCard {
                number: "03",
                content: stack_cards::CardContent::Letter('C'),
            },
            stack_cards::StackCard {
                number: "02",
                content: stack_cards::CardContent::Letter('B'),
            },
            stack_cards::StackCard {
                number: "01",
                content: stack_cards::CardContent::Letter('A'),
            },
        ];
        stack_cards::stack_cards(&mut cs, stack, &cards, s);

        let (op3, off3) = entrance(ui.ctx(), group, 3, s);
        let patch = layout.patch.translate(off3);
        let mut cp = ui.new_child(UiBuilder::new().max_rect(patch));
        cp.set_opacity(op3);
        panel::panel(
            &cp.painter().clone(),
            patch,
            theme::PANEL,
            px(theme::RADIUS_PANEL_MIN),
        );
        cp.painter().text(
            Pos2::new(patch.left() + px(24.0), patch.top() + px(24.0)),
            Align2::LEFT_TOP,
            "PATCH NOTES",
            FontId::new(px(14.0), theme::family_semibold()),
            theme::INK,
        );
        fan_cards::fan_cards(
            &mut cp,
            Rect::from_min_size(
                Pos2::new(patch.left() + px(24.0), patch.top() + px(70.0)),
                Vec2::new(patch.width() - px(24.0), patch.height() - px(80.0)),
            ),
            &['P', 'Q', 'R'],
            s,
        );
    }
}

fn entrance(ctx: &egui::Context, group: egui::Id, i: usize, s: f32) -> (f32, Vec2) {
    let (op, yoff) = anim::stagger_in(ctx, group, i, 60.0);
    (op, Vec2::new(0.0, theme::px(yoff, s)))
}
