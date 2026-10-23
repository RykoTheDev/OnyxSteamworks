use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, UiBuilder, Vec2};

use crate::app::App;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::theme;

#[derive(Default)]
struct Draft {
    enabled: bool,
    base_url: String,
    model: String,
    api_key: String,
    advisor_mb: u32,
    growth_pct: u32,
    loaded: bool,
}

pub struct AdvancedState {
    draft: Draft,
}

impl AdvancedState {
    pub fn new() -> Self {
        Self {
            draft: Draft::default(),
        }
    }

    pub fn reload(&mut self) {
        self.draft.loaded = false;
    }
}

fn load_into(draft: &mut Draft) {
    if draft.loaded {
        return;
    }
    let cfg = crate::config::load_settings().unwrap_or_default();
    draft.enabled = cfg.llm_enabled;
    draft.base_url = cfg.llm_base_url;
    draft.model = cfg.llm_model;
    draft.api_key = cfg.llm_api_key;
    draft.advisor_mb = if cfg.advisor_large_mb == 0 {
        100
    } else {
        cfg.advisor_large_mb
    };
    draft.growth_pct = if cfg.analytics_growth_pct == 0 {
        30
    } else {
        cfg.analytics_growth_pct
    };
    draft.loaded = true;
}

fn persist(draft: &Draft) {
    let mut cfg = crate::config::load_settings().unwrap_or_default();
    cfg.llm_enabled = draft.enabled;
    cfg.llm_base_url = draft.base_url.clone();
    cfg.llm_model = draft.model.clone();
    cfg.llm_api_key = draft.api_key.clone();
    cfg.advisor_large_mb = draft.advisor_mb;
    cfg.analytics_growth_pct = draft.growth_pct;
    if let Err(e) = crate::config::save_settings(&cfg) {
        eprintln!("could not save settings: {e}");
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let mut cu = ui.new_child(UiBuilder::new().max_rect(rect));
    cu.set_clip_rect(rect);
    egui::ScrollArea::vertical()
        .id_salt("advanced_scroll")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            let (canvas, _) = ui.allocate_exact_size(
                Vec2::new(rect.width() - px(16.0), px(730.0)),
                Sense::hover(),
            );
            ai_and_analytics(app, ui, canvas, s);
            let dev = Rect::from_min_max(
                Pos2::new(canvas.left(), canvas.top() + px(528.0)),
                Pos2::new(canvas.right(), canvas.bottom()),
            );
            super::developer::show(app, ui, dev, s);
        });
}

fn ai_and_analytics(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    load_into(&mut app.advanced_ui.draft);
    let draft = &mut app.advanced_ui.draft;

    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left(), rect.top()),
        "Patch notes AI",
        s,
    );
    ui.painter().text(
        Pos2::new(rect.left(), rect.top() + px(26.0)),
        Align2::LEFT_TOP,
        "Draft release notes with an OpenAI-compatible endpoint (Ollama, LM Studio, or a hosted API).",
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_MUTED,
    );

    let card = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + px(52.0)),
        Vec2::new(rect.width().min(px(560.0)), px(264.0)),
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

    let tog = Rect::from_min_size(
        Pos2::new(x, card.top() + px(20.0)),
        Vec2::new(px(44.0), px(24.0)),
    );
    let mut enabled = draft.enabled;
    if kit::toggle(ui, tog, &mut enabled, s).changed() {
        draft.enabled = enabled;
        persist(draft);
    }
    ui.painter().text(
        Pos2::new(tog.right() + px(12.0), tog.center().y),
        Align2::LEFT_CENTER,
        "Enable AI patch notes",
        FontId::new(px(13.0), theme::family_semibold()),
        theme::INK,
    );

    let row = |y: f32| Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, px(34.0)));
    let y0 = card.top() + px(74.0);
    field(
        ui,
        row(y0),
        "Base URL",
        "http://localhost:11434/v1",
        &mut draft.base_url,
        false,
        s,
    );
    field(
        ui,
        row(y0 + px(64.0)),
        "Model",
        "llama3.1",
        &mut draft.model,
        false,
        s,
    );
    field(
        ui,
        row(y0 + px(128.0)),
        "API key (optional)",
        "sk-... (leave blank for local servers)",
        &mut draft.api_key,
        true,
        s,
    );

    if ui.input(|i| i.pointer.any_released()) {
        persist(draft);
    }

    ui.painter().text(
        Pos2::new(card.left(), card.bottom() + px(18.0)),
        Align2::LEFT_TOP,
        "Off by default. When on, commit messages for the build are sent to this endpoint.",
        FontId::new(px(11.5), theme::family_medium()),
        theme::INK_MUTED,
    );

    let ay = card.bottom() + px(48.0);
    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left(), ay),
        "Analytics",
        s,
    );
    let acard = Rect::from_min_size(
        Pos2::new(rect.left(), ay + px(26.0)),
        Vec2::new(rect.width().min(px(560.0)), px(104.0)),
    );
    kit::bordered(
        &ui.painter().clone(),
        acard,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    let ax = acard.left() + px(20.0);
    let mut changed = false;
    changed |= stepper(
        ui,
        Pos2::new(ax, acard.top() + px(20.0)),
        "Flag large files over (MB)",
        &mut draft.advisor_mb,
        (10, 10, 2000),
        s,
    );
    changed |= stepper(
        ui,
        Pos2::new(ax, acard.top() + px(58.0)),
        "Alert when build grows by (%)",
        &mut draft.growth_pct,
        (5, 5, 300),
        s,
    );
    if changed {
        persist(draft);
    }
}

fn stepper(
    ui: &mut egui::Ui,
    at: Pos2,
    label: &str,
    value: &mut u32,
    bounds: (u32, u32, u32),
    s: f32,
) -> bool {
    let px = |v: f32| theme::px(v, s);
    let (step, min, max) = bounds;
    let (x, y) = (at.x, at.y);
    ui.painter().text(
        Pos2::new(x, y + px(15.0)),
        Align2::LEFT_CENTER,
        label,
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_DIM,
    );
    let bx = x + px(230.0);
    let minus = Rect::from_min_size(Pos2::new(bx, y), Vec2::splat(px(30.0)));
    let plus = Rect::from_min_size(
        Pos2::new(minus.right() + px(44.0), y),
        Vec2::splat(px(30.0)),
    );
    ui.painter().text(
        Pos2::new((minus.right() + plus.left()) / 2.0, y + px(15.0)),
        Align2::CENTER_CENTER,
        value.to_string(),
        FontId::new(px(13.5), theme::family_semibold()),
        theme::INK,
    );
    let mut changed = false;
    if Button::new("-", ButtonKind::Secondary)
        .show(ui, minus, s)
        .clicked()
        && *value > min
    {
        *value = value.saturating_sub(step).max(min);
        changed = true;
    }
    if Button::new("+", ButtonKind::Secondary)
        .show(ui, plus, s)
        .clicked()
        && *value < max
    {
        *value = (*value + step).min(max);
        changed = true;
    }
    changed
}

fn field(
    ui: &mut egui::Ui,
    row: Rect,
    label: &str,
    hint: &str,
    value: &mut String,
    mask: bool,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    ui.painter().text(
        Pos2::new(row.left(), row.top() - px(18.0)),
        Align2::LEFT_TOP,
        label,
        FontId::new(px(11.5), theme::family_label()),
        theme::INK_MUTED,
    );
    Input::new(value).hint(hint).mask(mask).show(ui, row, s);
}
