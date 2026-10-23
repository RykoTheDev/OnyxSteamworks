use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use super::timeline;
use crate::app::App;
use crate::config;
use crate::features::{maintenance, presets};
use crate::model::StepKind;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::theme;

const VARIABLES: &str =
    "Variables: {version}  {branch}  {build_number}  {git_commit}  {date}  {time}";

pub(super) fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32, ctx: &egui::Context) {
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
    app.pipeline_ui.sync(&app.projects);
    let running = app.pipeline_ui.running;

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
    let project_dd = Rect::from_min_size(rect.min, Vec2::new(px(220.0), px(38.0)));
    let mut project_sel = app.pipeline_ui.project_sel;
    kit::dropdown(ui, project_dd, &refs, &mut project_sel, s);
    if project_sel != app.pipeline_ui.project_sel && !running {
        app.pipeline_ui.project_sel = project_sel;
        app.pipeline_ui.sync(&app.projects);
    }

    if app.pipeline_ui.draft.pipelines.is_empty() {
        no_pipelines(app, ui, rect, s);
        return;
    }

    let pipeline_names: Vec<String> = app
        .pipeline_ui
        .draft
        .pipelines
        .iter()
        .map(|p| {
            if p.name.trim().is_empty() {
                "Untitled pipeline".to_string()
            } else {
                p.name.clone()
            }
        })
        .collect();
    let pipeline_refs: Vec<&str> = pipeline_names.iter().map(|x| x.as_str()).collect();
    let pipeline_dd = Rect::from_min_size(
        Pos2::new(project_dd.right() + px(10.0), rect.top()),
        Vec2::new(px(220.0), px(38.0)),
    );
    let mut pipeline_sel = app.pipeline_ui.pipeline_sel;
    kit::dropdown(ui, pipeline_dd, &pipeline_refs, &mut pipeline_sel, s);
    app.pipeline_ui.choose_pipeline(pipeline_sel);

    let new_btn = Rect::from_min_size(
        Pos2::new(pipeline_dd.right() + px(10.0), rect.top()),
        Vec2::new(px(132.0), px(38.0)),
    );
    if Button::new("New pipeline", ButtonKind::Secondary)
        .icon(theme::icon::PLUS)
        .show(ui, new_btn, s)
        .clicked()
    {
        app.pipeline_ui.add_pipeline();
    }

    let run_rect = Rect::from_min_size(
        Pos2::new(rect.right() - px(120.0), rect.top()),
        Vec2::new(px(120.0), px(38.0)),
    );
    if running {
        if Button::new("Cancel", ButtonKind::Danger)
            .icon(theme::icon::X)
            .show(ui, run_rect, s)
            .clicked()
        {
            app.pipeline_ui.cancel();
        }
    } else if Button::new("Run", ButtonKind::Primary)
        .icon(theme::icon::PLAY)
        .enabled(!app.pipeline_ui.steps().is_empty() && !app.scheduler.busy())
        .disabled_reason(if app.scheduler.busy() {
            "A scheduled run is in progress"
        } else {
            "Add a step first"
        })
        .show(ui, run_rect, s)
        .clicked()
    {
        app.pipeline_ui.start(ctx, 0);
    }
    let dirty = app.pipeline_ui.dirty();
    let save_rect = Rect::from_min_size(
        Pos2::new(run_rect.left() - px(98.0), rect.top()),
        Vec2::new(px(88.0), px(38.0)),
    );
    if Button::new("Save", ButtonKind::Secondary)
        .enabled(dirty)
        .show(ui, save_rect, s)
        .clicked()
        && dirty
    {
        save(app);
    }
    if dirty
        && Button::new("Revert", ButtonKind::Ghost)
            .show(
                ui,
                Rect::from_min_size(
                    Pos2::new(save_rect.left() - px(90.0), rect.top()),
                    Vec2::new(px(80.0), px(38.0)),
                ),
                s,
            )
            .clicked()
    {
        app.pipeline_ui.revert();
    }

    let row2 = rect.top() + px(48.0);
    let sel = app.pipeline_ui.pipeline_sel;
    if let Some(pipe) = app.pipeline_ui.draft.pipelines.get_mut(sel) {
        Input::new(&mut pipe.name).hint("Pipeline name").show(
            ui,
            Rect::from_min_size(Pos2::new(rect.left(), row2), Vec2::new(px(220.0), px(36.0))),
            s,
        );
    }
    let mut labels = vec!["+ Add step"];
    labels.extend(StepKind::ALL.iter().map(|k| k.label()));
    let mut add_sel = 0usize;
    let add_rect = Rect::from_min_size(
        Pos2::new(rect.left() + px(230.0), row2),
        Vec2::new(px(220.0), px(36.0)),
    );
    kit::dropdown(ui, add_rect, &labels, &mut add_sel, s);
    if add_sel > 0 {
        app.pipeline_ui.add_step(StepKind::ALL[add_sel - 1]);
    }
    let engine = presets::detect(&maintenance::resolve(&app.pipeline_ui.draft).content_root);
    let next_x = add_rect.right() + px(10.0);
    if let Some(engine) = engine {
        let label = format!("Use {} template", engine.label());
        if Button::new(&label, ButtonKind::Ghost)
            .icon(theme::icon::SPARKLE)
            .show(
                ui,
                Rect::from_min_size(Pos2::new(next_x, row2), Vec2::new(px(180.0), px(36.0))),
                s,
            )
            .clicked()
        {
            app.pipeline_ui.apply_preset(engine);
        }
    }
    if Button::new("Delete pipeline", ButtonKind::Danger)
        .icon(theme::icon::TRASH)
        .enabled(!running)
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.right() - px(150.0), row2),
                Vec2::new(px(150.0), px(36.0)),
            ),
            s,
        )
        .clicked()
    {
        app.pipeline_ui.delete_pipeline();
    }
    ui.painter().text(
        Pos2::new(rect.left(), row2 + px(48.0)),
        Align2::LEFT_CENTER,
        VARIABLES,
        FontId::new(px(11.5), theme::family_medium()),
        theme::INK_FAINT,
    );

    let mut top = row2 + px(66.0);
    if let Some((_, message)) = app.pipeline_ui.awaiting.clone() {
        let banner = Rect::from_min_size(
            Pos2::new(rect.left(), top),
            Vec2::new(rect.width(), px(44.0)),
        );
        kit::bordered(
            &ui.painter().clone(),
            banner,
            theme::with_alpha(theme::WARNING, 18),
            theme::with_alpha(theme::WARNING, 120),
            px(theme::RADIUS_FIELD),
        );
        ui.painter().text(
            Pos2::new(banner.left() + px(14.0), banner.center().y),
            Align2::LEFT_CENTER,
            if message.is_empty() {
                "Waiting for approval. Continue when ready.".to_string()
            } else {
                message
            },
            FontId::new(px(12.5), theme::family_medium()),
            theme::INK,
        );
        if Button::new("Approve", ButtonKind::Primary)
            .icon(theme::icon::CHECK)
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

fn no_pipelines(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let body = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + px(48.0)), rect.max);
    kit::empty_state(
        ui,
        body,
        theme::icon::FLOW,
        "No pipelines yet",
        "A pipeline runs steps in order, like building the game and uploading it to Steam.",
        s,
    );
    let btn = Rect::from_center_size(
        Pos2::new(body.center().x, body.center().y + px(64.0)),
        Vec2::new(px(180.0), px(40.0)),
    );
    if Button::new("Create pipeline", ButtonKind::Primary)
        .icon(theme::icon::PLUS)
        .show(ui, btn, s)
        .clicked()
    {
        app.pipeline_ui.add_pipeline();
    }
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
    let title = if app.pipeline_ui.running {
        "Run log (running)"
    } else {
        "Run log"
    };
    ui.painter().text(
        Pos2::new(rect.left() + px(14.0), rect.top() + px(16.0)),
        Align2::LEFT_TOP,
        title,
        FontId::new(px(12.0), theme::family_label()),
        theme::INK_MUTED,
    );
    let inner = Rect::from_min_max(
        Pos2::new(rect.left() + px(14.0), rect.top() + px(40.0)),
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
    let pipelines = app.pipeline_ui.draft.pipelines.clone();
    let id = app.pipeline_ui.draft.id.clone();
    let mut project = app
        .projects
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .unwrap_or_else(|| app.pipeline_ui.draft.clone());
    project.pipelines = pipelines;
    match config::save_project(&project) {
        Ok(()) => {
            app.projects = config::list_projects().unwrap_or_default();
            app.pipeline_ui.mark_saved();
            app.toasts.push(kit::ToastLevel::Success, "Pipelines saved");
        }
        Err(e) => {
            eprintln!("save failed: {e}");
            app.toasts.push(kit::ToastLevel::Error, "Save failed");
        }
    }
}
