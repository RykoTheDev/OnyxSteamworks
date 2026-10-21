mod card;
mod queue;
mod queue_cards;
mod schedule;
mod state;
mod timeline;

pub use queue::QueueState;
pub use state::PipelineState;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use crate::app::{App, Page};
use crate::config;
use crate::features::presets;
use crate::model::StepKind;
use crate::ui::kit::{Button, ButtonKind};
use crate::ui::layout::Layout;
use crate::ui::{kit, nav, pages, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let s = layout.scale;
    let ctx = ui.ctx().clone();
    for toast in app.pipeline_ui.drain(&ctx) {
        app.toasts.push(kit::ToastLevel::Success, toast);
    }
    let idx = nav::index(Page::Pipelines);
    let tabs = nav::ITEMS[idx].sub_tabs;
    let mut active = app.sub_tabs[idx];
    let body = pages::hero_and_tabs(ui, layout.content, s, "PIPELINES", tabs, &mut active);
    app.sub_tabs[idx] = active;

    match active {
        0 => pipelines_tab(app, ui, body, s, &ctx),
        1 => queue::show(app, ui, body, s),
        _ => schedule::show(app, ui, body, s),
    }
}

fn pipelines_tab(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32, ctx: &egui::Context) {
    let px = |v: f32| theme::px(v, s);
    if app.projects.is_empty() {
        kit::empty_state(
            ui,
            rect,
            theme::icon::FLOW,
            "No projects",
            "Create a project first, then build a pipeline for it.",
            s,
        );
        return;
    }
    app.pipeline_ui
        .sync(&app.projects, app.pipeline_ui.project_sel);

    let names: Vec<String> = app
        .projects
        .iter()
        .map(|p| {
            if p.name.is_empty() {
                "Untitled".to_string()
            } else {
                p.name.clone()
            }
        })
        .collect();
    let refs: Vec<&str> = names.iter().map(|x| x.as_str()).collect();
    let dd = Rect::from_min_size(rect.min, Vec2::new(px(236.0), px(40.0)));
    kit::dropdown(ui, dd, &refs, &mut app.pipeline_ui.project_sel, s);

    let engine = presets::detect(&app.pipeline_ui.draft.content_root);
    if let Some(engine) = engine {
        ui.painter().text(
            Pos2::new(dd.right() + px(16.0), dd.center().y),
            Align2::LEFT_CENTER,
            format!("{} project detected", engine.label()),
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_DIM,
        );
        if Button::new("APPLY PRESET", ButtonKind::Secondary)
            .show(
                ui,
                Rect::from_min_size(
                    Pos2::new(dd.right() + px(170.0), dd.top()),
                    Vec2::new(px(130.0), px(40.0)),
                ),
                s,
            )
            .clicked()
        {
            app.pipeline_ui.apply_preset(engine);
        }
    }

    let running = app.pipeline_ui.running;
    let run_rect = Rect::from_min_size(
        Pos2::new(rect.right() - px(130.0), rect.top()),
        Vec2::new(px(130.0), px(40.0)),
    );
    if running {
        if Button::new("CANCEL", ButtonKind::Danger)
            .icon(theme::icon::X)
            .show(ui, run_rect, s)
            .clicked()
        {
            app.pipeline_ui.cancel();
        }
    } else if Button::new("RUN", ButtonKind::Primary)
        .icon(theme::icon::PLAY)
        .enabled(!app.pipeline_ui.draft.pipeline.is_empty())
        .show(ui, run_rect, s)
        .clicked()
    {
        app.pipeline_ui.start(ctx, 0);
    }
    let dirty = app.pipeline_ui.dirty();
    if Button::new("SAVE", ButtonKind::Secondary)
        .enabled(dirty)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(run_rect.left() - px(100.0), rect.top()),
                Vec2::new(px(90.0), px(40.0)),
            ),
            s,
        )
        .clicked()
        && dirty
    {
        save(app);
    }

    let add_labels = [
        "+ Add step",
        "Upload",
        "Run command",
        "Clean",
        "Backup",
        "Notify",
        "Confirm",
    ];
    let mut add_sel = 0usize;
    kit::dropdown(
        ui,
        Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + px(48.0)),
            Vec2::new(px(170.0), px(36.0)),
        ),
        &add_labels,
        &mut add_sel,
        s,
    );
    if add_sel > 0 {
        app.pipeline_ui.add_step(StepKind::ALL[add_sel - 1]);
    }
    ui.painter().text(
        Pos2::new(rect.left() + px(186.0), rect.top() + px(66.0)),
        Align2::LEFT_CENTER,
        presets::TEMPLATE_NOTE,
        FontId::new(px(11.5), theme::family_medium()),
        theme::INK_MUTED,
    );

    let mut top = rect.top() + px(96.0);
    if let Some((_, message)) = app.pipeline_ui.awaiting.clone() {
        let banner = Rect::from_min_size(
            Pos2::new(rect.left(), top),
            Vec2::new(rect.width(), px(44.0)),
        );
        kit::bordered(
            &ui.painter().clone(),
            banner,
            theme::FIELD,
            theme::WARNING,
            px(theme::RADIUS_FIELD),
        );
        ui.painter().text(
            Pos2::new(banner.left() + px(14.0), banner.center().y),
            Align2::LEFT_CENTER,
            if message.is_empty() {
                "Paused. Continue when ready.".to_string()
            } else {
                message
            },
            FontId::new(px(12.5), theme::family_medium()),
            theme::INK,
        );
        if Button::new("CONTINUE", ButtonKind::Primary)
            .show(
                ui,
                Rect::from_min_size(
                    Pos2::new(banner.right() - px(130.0), banner.top() + px(4.0)),
                    Vec2::new(px(120.0), px(36.0)),
                ),
                s,
            )
            .clicked()
        {
            app.pipeline_ui.confirm();
        }
        top += px(54.0);
    }

    let log_w = px(320.0);
    let timeline_rect = Rect::from_min_max(
        Pos2::new(rect.left(), top),
        Pos2::new(rect.right() - log_w - px(20.0), rect.bottom()),
    );
    let log_rect = Rect::from_min_max(Pos2::new(rect.right() - log_w, top), rect.max);
    timeline::show(app, ui, timeline_rect, s, ctx);
    run_log(app, ui, log_rect, s);
}

fn run_log(app: &App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::BG,
        theme::LINE,
        px(theme::RADIUS_CARD),
    );
    ui.painter().text(
        Pos2::new(rect.left() + px(14.0), rect.top() + px(16.0)),
        Align2::LEFT_TOP,
        "RUN LOG",
        FontId::new(px(11.0), theme::family_label()),
        theme::INK_MUTED,
    );
    let inner = Rect::from_min_max(
        Pos2::new(rect.left() + px(14.0), rect.top() + px(38.0)),
        Pos2::new(rect.right() - px(10.0), rect.bottom() - px(12.0)),
    );
    let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    cu.set_clip_rect(inner);
    egui::ScrollArea::vertical()
        .id_salt("pipeline_runlog")
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(&mut cu, |ui| {
            if app.pipeline_ui.logs.is_empty() {
                ui.label(
                    egui::RichText::new("No run output yet.")
                        .font(FontId::new(px(12.0), theme::family_medium()))
                        .color(theme::INK_MUTED),
                );
            }
            for line in &app.pipeline_ui.logs {
                ui.label(
                    egui::RichText::new(line)
                        .monospace()
                        .size(px(11.5))
                        .color(theme::INK_DIM),
                );
            }
        });
}

fn save(app: &mut App) {
    match config::save_project(&app.pipeline_ui.draft) {
        Ok(()) => {
            app.projects = config::list_projects().unwrap_or_default();
            app.pipeline_ui.mark_saved();
            app.toasts.push(kit::ToastLevel::Success, "Pipeline saved");
        }
        Err(e) => {
            eprintln!("save failed: {e}");
            app.toasts.push(kit::ToastLevel::Error, "Save failed");
        }
    }
}
