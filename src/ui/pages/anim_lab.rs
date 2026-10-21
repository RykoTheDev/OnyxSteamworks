use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Vec2};

mod feedback;
mod forms;
mod widgets;

use crate::app::{App, Page};
use crate::ui::bento::{panel, rail};
use crate::ui::{anim, theme};

#[derive(Default)]
pub struct LabState {
    pub search: String,
    pub prompt: String,
    pub rail_page: Page,
    pub chips_single: Vec<usize>,
    pub chips_multi: Vec<usize>,
    pub segment: usize,
    pub toggle_on: bool,
    pub checked: bool,
    pub input: String,
    pub secret: String,
    pub error_text: String,
    pub view: usize,
    pub dropdown_sel: usize,
    pub tab_active: usize,
    pub progress: f32,
    pub modal_open: bool,
    pub toasts: crate::ui::bento::toast::Toasts,
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let s = theme::scale(ui.available_size());
    let px = |v: f32| theme::px(v, s);
    let area = ui.max_rect();
    let pad = px(theme::SPACE_LG);

    {
        let painter = ui.painter().clone();
        painter.text(
            area.left_top() + Vec2::new(pad, px(8.0)),
            Align2::LEFT_TOP,
            "ANIMATION LAB",
            FontId::new(px(16.0), theme::family_semibold()),
            theme::PANEL,
        );
    }

    let tabs = ["PRIMITIVES", "FORMS", "FEEDBACK"];
    for (i, name) in tabs.iter().enumerate() {
        let tab = Rect::from_min_size(
            Pos2::new(
                area.left() + pad + px(180.0) + i as f32 * px(130.0),
                area.top() + px(6.0),
            ),
            Vec2::new(px(120.0), px(28.0)),
        );
        if ui
            .interact(tab, ui.scope_id().with(("lab_tab", i)), Sense::click())
            .clicked()
        {
            app.lab.view = i;
        }
        let on = app.lab.view == i;
        let painter = ui.painter();
        painter.rect_filled(
            tab,
            theme::corner(px(14.0)),
            if on { theme::PANEL } else { theme::BG },
        );
        painter.rect_stroke(
            tab,
            theme::corner(px(14.0)),
            egui::Stroke::new(1.0, theme::PANEL),
            egui::StrokeKind::Inside,
        );
        painter.text(
            tab.center(),
            Align2::CENTER_CENTER,
            *name,
            FontId::new(px(12.0), theme::family_medium()),
            if on { theme::INK } else { theme::PANEL },
        );
    }

    let top = area.top() + px(52.0);
    let c1 = area.left() + pad;
    if app.lab.view == 0 {
        let col = px(330.0);
        let c2 = c1 + col + pad;
        let c3 = c2 + col + pad;
        let w3 = (area.right() - px(104.0) - pad - c3).max(px(300.0));
        anim_demos(app, ui, &ctx, s, c1, top, col);
        widgets::primitive_demos(app, ui, s, c2, top, px(300.0));
        widgets::primitives_u5(app, ui, s, c3, top, w3);
        rail_demo(app, ui, s, area);
    } else if app.lab.view == 1 {
        forms::form_demos(
            app,
            ui,
            s,
            c1,
            top,
            (area.width() - pad * 2.0).max(px(320.0)),
        );
    } else {
        feedback::feedback_demos(
            app,
            ui,
            s,
            c1,
            top,
            (area.width() - pad * 2.0).max(px(320.0)),
        );
    }
}

fn anim_demos(
    _app: &mut App,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    s: f32,
    left: f32,
    top: f32,
    col_w: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let painter = ui.painter().clone();
    let mut y = top;

    y = label(&painter, left, y, px(12.0), "HOVER + PRESS");
    let card = Rect::from_min_size(Pos2::new(left, y), Vec2::new(px(220.0), px(96.0)));
    let card_id = ui.scope_id().with("hover_card");
    let resp = ui.interact(card, card_id, Sense::click());
    let hover = anim::anim_bool(ctx, card_id, resp.hovered(), 0.14, anim::ease_out_cubic);
    let press = anim::anim_bool(
        ctx,
        card_id.with("p"),
        resp.is_pointer_button_down_on(),
        0.09,
        anim::ease_out_cubic,
    );
    let scaled = anim::scale_rect(card, 1.0 + 0.03 * hover - 0.06 * press);
    panel::panel(&painter, scaled, theme::PANEL, px(theme::RADIUS_CARD));
    painter.text(
        scaled.center(),
        Align2::CENTER_CENTER,
        "HOVER ME",
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK,
    );
    y += px(96.0) + px(theme::SPACE_MD);

    y = label(&painter, left, y, px(12.0), "SPRING (CLICK)");
    let zone = Rect::from_min_size(Pos2::new(left, y), Vec2::new(col_w, px(120.0)));
    let zresp = ui.interact(zone, ui.scope_id().with("spring"), Sense::click());
    let key = ui.scope_id().with("spring_start");
    if zresp.clicked() {
        let now = ctx.input(|i| i.time);
        ctx.data_mut(|d| d.insert_temp(key, now));
    }
    let start = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(key, || f64::NEG_INFINITY));
    let scale = if start.is_finite() {
        let elapsed = (ctx.input(|i| i.time) - start) as f32;
        let settle = anim::spring(elapsed, 14.0);
        if settle < 0.999 {
            ctx.request_repaint();
        }
        1.0 + 0.5 * (1.0 - settle)
    } else {
        1.0
    };
    panel::panel(&painter, zone, theme::PANEL, px(theme::RADIUS_CARD));
    painter.circle_filled(zone.center(), px(30.0) * scale, theme::INK);
    y += px(120.0) + px(theme::SPACE_MD);

    y = label(&painter, left, y, px(12.0), "STAGGER (CLICK TO REPLAY)");
    let group = ui.scope_id().with("stagger");
    let list = Rect::from_min_size(Pos2::new(left, y), Vec2::new(col_w, px(160.0)));
    if ui
        .interact(list, group.with("replay"), Sense::click())
        .clicked()
    {
        anim::reset_stagger(ctx, group);
    }
    let row_h = px(24.0);
    let gap = px(6.0);
    for i in 0..5usize {
        let (opacity, y_ref) = anim::stagger_in(ctx, group, i, 70.0);
        let row = Rect::from_min_size(
            Pos2::new(
                list.left(),
                list.top() + i as f32 * (row_h + gap) + px(y_ref),
            ),
            Vec2::new(col_w, row_h),
        );
        panel::panel(
            &painter,
            row,
            anim::with_opacity(theme::PANEL, opacity),
            px(12.0),
        );
    }

    let toggle = Rect::from_min_size(
        Pos2::new(left, y + px(170.0)),
        Vec2::new(px(220.0), px(26.0)),
    );
    if ui
        .interact(toggle, ui.scope_id().with("reduce"), Sense::click())
        .clicked()
    {
        anim::set_reduce_motion(!anim::reduce_motion());
    }
    let state = if anim::reduce_motion() { "ON" } else { "OFF" };
    painter.rect_stroke(
        toggle,
        theme::corner(px(13.0)),
        egui::Stroke::new(1.0, theme::PANEL),
        egui::StrokeKind::Inside,
    );
    painter.text(
        toggle.center(),
        Align2::CENTER_CENTER,
        format!("REDUCE MOTION: {state}"),
        FontId::new(px(12.0), theme::family_medium()),
        theme::PANEL,
    );
}

fn rail_demo(app: &mut App, ui: &mut egui::Ui, s: f32, area: Rect) {
    let px = |v: f32| theme::px(v, s);
    let rail_rect = Rect::from_min_size(
        Pos2::new(area.right() - px(92.0) - px(8.0), area.top() + px(140.0)),
        Vec2::new(px(92.0), (area.height() - px(220.0)).min(px(560.0))),
    );
    if let Some(rail::RailAction::Navigate(page)) = rail::rail(ui, rail_rect, app.lab.rail_page, s)
    {
        app.lab.rail_page = page;
    }
}

fn label(painter: &egui::Painter, x: f32, y: f32, size: f32, text: &str) -> f32 {
    painter.text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        text,
        FontId::new(size, theme::family_medium()),
        theme::INK_MUTED,
    );
    y + size + 6.0
}
