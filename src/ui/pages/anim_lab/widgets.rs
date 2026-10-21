use eframe::egui::{self, Pos2, Rect, Sense, Vec2};

use super::label;
use crate::app::App;
use crate::ui::bento::{
    chip, fan_cards, image, notch_button, panel, prompt_bar, search_pill, segment_progress,
    skeleton, stack_cards, status,
};
use crate::ui::theme;

pub(super) fn primitive_demos(
    app: &mut App,
    ui: &mut egui::Ui,
    s: f32,
    left: f32,
    top: f32,
    w: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let mut y = top;

    y = label(&ui.painter().clone(), left, y, px(12.0), "SEARCH PILL");
    let search = Rect::from_min_size(Pos2::new(left, y), Vec2::new(w.min(px(266.0)), px(60.0)));
    search_pill::search_pill(ui, search, &mut app.lab.search, s);
    y += px(60.0) + px(theme::SPACE_MD);

    y = label(&ui.painter().clone(), left, y, px(12.0), "PROMPT BAR");
    let prompt = Rect::from_min_size(Pos2::new(left, y), Vec2::new(w.min(px(278.0)), px(48.0)));
    if prompt_bar::prompt_bar(ui, prompt, &mut app.lab.prompt, s) {
        app.lab.prompt.clear();
    }
    y += px(48.0) + px(theme::SPACE_MD);

    y = label(&ui.painter().clone(), left, y, px(12.0), "PANEL + CUTOUT");
    let base = Rect::from_min_size(Pos2::new(left, y), Vec2::new(w.min(px(260.0)), px(120.0)));
    let painter = ui.painter().clone();
    panel::panel(&painter, base, theme::PANEL, px(theme::RADIUS_CARD));
    let bite = Rect::from_min_size(
        Pos2::new(base.right() - px(70.0), base.bottom() - px(70.0)),
        Vec2::splat(px(90.0)),
    );
    panel::cutout(&painter, bite, px(theme::RADIUS_CARD));
    y += px(120.0) + px(theme::SPACE_MD);

    y = label(
        &ui.painter().clone(),
        left,
        y,
        px(12.0),
        "IMAGE PLACEHOLDER",
    );
    let img = Rect::from_min_size(Pos2::new(left, y), Vec2::new(px(120.0), px(120.0)));
    image::image_or_placeholder(ui, img, None, 'K', px(theme::RADIUS_CARD), s);

    let notch_y = y + px(130.0);
    label(
        &ui.painter().clone(),
        left,
        notch_y,
        px(12.0),
        "NOTCH BUTTON",
    );
    notch_button::notch_button(ui, Pos2::new(left + px(60.0), notch_y + px(70.0)), s);
}

pub(super) fn primitives_u5(app: &mut App, ui: &mut egui::Ui, s: f32, left: f32, top: f32, w: f32) {
    let px = |v: f32| theme::px(v, s);
    let rc =
        |x: f32, y: f32, ww: f32, hh: f32| Rect::from_min_size(Pos2::new(x, y), Vec2::new(ww, hh));
    let labels = ["ALL", "NEW", "BETA", "LIVE", "DRAFT"];
    let mut y = top;

    y = label(&ui.painter().clone(), left, y, px(12.0), "CHIPS (SINGLE)");
    chip::chip_grid(
        ui,
        rc(left, y, w, px(60.0)),
        &labels,
        &mut app.lab.chips_single,
        false,
        s,
    );
    y += px(66.0);

    y = label(&ui.painter().clone(), left, y, px(12.0), "CHIPS (MULTI)");
    chip::chip_grid(
        ui,
        rc(left, y, w, px(60.0)),
        &labels,
        &mut app.lab.chips_multi,
        true,
        s,
    );
    y += px(66.0);

    y = label(&ui.painter().clone(), left, y, px(12.0), "SEGMENT (CLICK)");
    let seg = rc(left, y, px(170.0), px(16.0));
    if ui
        .interact(seg, ui.scope_id().with("seg_demo"), Sense::click())
        .clicked()
    {
        app.lab.segment = (app.lab.segment + 1) % 5;
    }
    segment_progress::segment_progress(ui, Pos2::new(left, y + px(6.0)), 5, app.lab.segment, s);
    y += px(30.0);

    y = label(&ui.painter().clone(), left, y, px(12.0), "PILL + TAG");
    status::pill(
        ui,
        rc(left, y, px(120.0), px(28.0)),
        "READY",
        status::Status::Success,
        s,
    );
    status::tag(
        ui,
        rc(left + px(132.0), y, px(92.0), px(24.0)),
        "WARN",
        status::Status::Warning,
        s,
    );
    status::tag(
        ui,
        rc(left + px(232.0), y, px(92.0), px(24.0)),
        "FAIL",
        status::Status::Danger,
        s,
    );
    y += px(40.0);

    y = label(&ui.painter().clone(), left, y, px(12.0), "SKELETON");
    skeleton::skeleton(ui, rc(left, y, w.min(px(260.0)), px(16.0)), px(8.0));
    skeleton::skeleton(
        ui,
        rc(left, y + px(24.0), w.min(px(180.0)), px(16.0)),
        px(8.0),
    );
    y += px(56.0);

    label(&ui.painter().clone(), left, y, px(12.0), "STACK CARDS");
    let cards = [
        stack_cards::StackCard {
            number: "01",
            content: stack_cards::CardContent::Letter('A'),
        },
        stack_cards::StackCard {
            number: "02",
            content: stack_cards::CardContent::Letter('B'),
        },
        stack_cards::StackCard {
            number: "03",
            content: stack_cards::CardContent::Letter('C'),
        },
    ];
    stack_cards::stack_cards(ui, rc(left, y + px(20.0), px(150.0), px(170.0)), &cards, s);

    label(
        &ui.painter().clone(),
        left + px(172.0),
        y,
        px(12.0),
        "FAN CARDS",
    );
    fan_cards::fan_cards(
        ui,
        rc(left + px(172.0), y + px(20.0), w - px(172.0), px(220.0)),
        &['K', 'L', 'M'],
        s,
    );
}
