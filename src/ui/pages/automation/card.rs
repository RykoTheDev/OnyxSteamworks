use eframe::egui::{self, Align2, FontId, Frame, Pos2, Rect, Vec2};

use crate::features::presets::{self, Engine};
use crate::model::{PipelineStep, Project, RunIf, StepKind};
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

pub(super) fn extra_height(project: &Project, step: &PipelineStep) -> f32 {
    let mut h = 58.0 + 48.0 + 58.0;
    match step.kind {
        StepKind::BuildGame => {
            h += 58.0 * 4.0 + 104.0;
            if presets::step_engine(project, step) == Some(Engine::Godot) {
                h += 58.0;
            }
        }
        StepKind::RunCommand => h += 58.0 * 3.0 + 92.0,
        StepKind::Clean => h += 30.0 + step.targets.len() as f32 * 34.0 + 36.0,
        StepKind::Notify | StepKind::Confirm => h += 58.0,
        StepKind::Upload | StepKind::Backup => h += 26.0,
    }
    h
}

fn lbl(ui: &egui::Ui, x: f32, y: f32, text: &str, s: f32) {
    ui.painter().text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        text,
        FontId::new(theme::px(11.0, s), theme::family_label()),
        theme::INK_MUTED,
    );
}

pub(super) fn fields(
    step: &mut PipelineStep,
    project: &Project,
    ui: &mut egui::Ui,
    rect: Rect,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let x = rect.left();
    let w = rect.width();
    let mut y = rect.top();

    lbl(ui, x, y, "Step name", s);
    Input::new(&mut step.name).hint("Step name").show(
        ui,
        Rect::from_min_size(
            Pos2::new(x, y + px(16.0)),
            Vec2::new(w.min(px(280.0)), px(36.0)),
        ),
        s,
    );
    y += px(62.0);

    kit::toggle(
        ui,
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(42.0), px(22.0))),
        &mut step.enabled,
        s,
    );
    text(ui, x + px(50.0), y + px(11.0), "Enabled", s);
    kit::toggle(
        ui,
        Rect::from_min_size(Pos2::new(x + px(150.0), y), Vec2::new(px(42.0), px(22.0))),
        &mut step.continue_on_failure,
        s,
    );
    text(ui, x + px(200.0), y + px(11.0), "Continue on failure", s);
    let to = Rect::from_min_size(
        Pos2::new(rect.right() - px(130.0), y - px(2.0)),
        Vec2::new(px(130.0), px(26.0)),
    );
    ui.new_child(egui::UiBuilder::new().max_rect(to)).add(
        egui::DragValue::new(&mut step.timeout_secs)
            .range(0..=86_400)
            .suffix(" s timeout"),
    );
    y += px(44.0);

    lbl(ui, x, y, "Run this step", s);
    let labels: Vec<&str> = RunIf::ALL.iter().map(|r| r.label()).collect();
    let mut sel = RunIf::ALL
        .iter()
        .position(|r| *r == step.run_if)
        .unwrap_or(0);
    kit::dropdown(
        ui,
        Rect::from_min_size(
            Pos2::new(x, y + px(16.0)),
            Vec2::new(w.min(px(280.0)), px(36.0)),
        ),
        &labels,
        &mut sel,
        s,
    );
    step.run_if = RunIf::ALL[sel];
    let hint = match step.run_if {
        RunIf::Always => "Runs unless an earlier step failed.",
        RunIf::PreviousFailed => "Use for alerts or clean-up after a failure.",
        RunIf::FilesChanged => "Skipped when the depot files match the last upload.",
    };
    text(ui, x + w.min(px(280.0)) + px(14.0), y + px(26.0), hint, s);
    y += px(58.0);

    match step.kind {
        StepKind::BuildGame => build_game_fields(step, project, ui, x, w, y, s),
        StepKind::RunCommand => run_command_fields(step, ui, x, w, y, s),
        StepKind::Clean => clean_fields(step, ui, x, w, y, s),
        StepKind::Notify | StepKind::Confirm => {
            lbl(ui, x, y, "Message", s);
            Input::new(&mut step.message).hint("Message").show(
                ui,
                Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(w, px(36.0))),
                s,
            );
        }
        StepKind::Upload => text(
            ui,
            x,
            y + px(6.0),
            "Generates the VDFs and uploads with SteamCMD. Authorize the account on the Build page first.",
            s,
        ),
        StepKind::Backup => text(
            ui,
            x,
            y + px(6.0),
            "Zips the content root into the backup folder set in Settings.",
            s,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_game_fields(
    step: &mut PipelineStep,
    project: &Project,
    ui: &mut egui::Ui,
    x: f32,
    w: f32,
    mut y: f32,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let half = (w - px(12.0)) / 2.0;

    lbl(ui, x, y, "Engine", s);
    let mut engines = vec!["Auto-detect"];
    engines.extend(Engine::ALL.iter().map(|e| e.label()));
    let mut engine_sel = Engine::from_label(&step.engine)
        .and_then(|e| Engine::ALL.iter().position(|x| *x == e))
        .map(|i| i + 1)
        .unwrap_or(0);
    kit::dropdown(
        ui,
        Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(half, px(36.0))),
        &engines,
        &mut engine_sel,
        s,
    );
    step.engine = if engine_sel == 0 {
        String::new()
    } else {
        Engine::ALL[engine_sel - 1].label().to_string()
    };
    lbl(ui, x + half + px(12.0), y, "Platform", s);
    let mut platform_sel = presets::PLATFORMS
        .iter()
        .position(|p| p.eq_ignore_ascii_case(step.platform.trim()))
        .unwrap_or(0);
    kit::dropdown(
        ui,
        Rect::from_min_size(
            Pos2::new(x + half + px(12.0), y + px(16.0)),
            Vec2::new(half, px(36.0)),
        ),
        &presets::PLATFORMS,
        &mut platform_sel,
        s,
    );
    step.platform = presets::PLATFORMS[platform_sel].to_string();
    y += px(58.0);

    let engine = presets::step_engine(project, step);
    lbl(ui, x, y, "Engine executable", s);
    Input::new(&mut step.exe)
        .hint(
            engine
                .map(|e| e.exe_hint())
                .unwrap_or("Unity.exe, RunUAT.bat or godot"),
        )
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(w, px(36.0))),
            s,
        );
    y += px(58.0);

    lbl(ui, x, y, "Engine project folder", s);
    Input::new(&mut step.working_dir)
        .hint(". (content root)")
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(half, px(36.0))),
            s,
        );
    lbl(ui, x + half + px(12.0), y, "Output folder", s);
    Input::new(&mut step.output)
        .hint("First depot folder")
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(x + half + px(12.0), y + px(16.0)),
                Vec2::new(half, px(36.0)),
            ),
            s,
        );
    y += px(58.0);

    if engine == Some(Engine::Godot) {
        lbl(ui, x, y, "Export preset", s);
        Input::new(&mut step.export_preset)
            .hint(presets::default_export_preset(&step.platform))
            .show(
                ui,
                Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(half, px(36.0))),
                s,
            );
        y += px(58.0);
    }

    lbl(ui, x, y, "Extra arguments", s);
    Input::new(&mut step.args)
        .hint("Appended to the generated command")
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(w, px(36.0))),
            s,
        );
    y += px(58.0);

    lbl(ui, x, y, "Command", s);
    let preview = Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(w, px(82.0)));
    kit::bordered(
        &ui.painter().clone(),
        preview,
        theme::BG,
        theme::LINE_SOFT,
        px(theme::RADIUS_FIELD),
    );
    let (body, color) = match presets::build_command(project, step) {
        Ok(cmd) => (cmd.display(), theme::INK_DIM),
        Err(e) => (e, theme::WARNING),
    };
    let inner = preview.shrink(px(10.0));
    let galley = ui
        .painter()
        .layout(body, FontId::monospace(px(11.5)), color, inner.width());
    ui.painter()
        .with_clip_rect(inner)
        .galley(inner.min, galley, color);
}

fn run_command_fields(
    step: &mut PipelineStep,
    ui: &mut egui::Ui,
    x: f32,
    w: f32,
    mut y: f32,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    lbl(ui, x, y, "Executable", s);
    Input::new(&mut step.exe)
        .hint("godot, Unity.exe, RunUAT.bat...")
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(x, y + px(16.0)),
                Vec2::new(w.min(px(300.0)), px(36.0)),
            ),
            s,
        );
    y += px(58.0);
    lbl(ui, x, y, "Arguments", s);
    Input::new(&mut step.args)
        .hint("--headless --path . --export-release ...")
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(w, px(36.0))),
            s,
        );
    y += px(58.0);
    lbl(ui, x, y, "Working dir", s);
    Input::new(&mut step.working_dir)
        .hint(". (content root)")
        .show(
            ui,
            Rect::from_min_size(
                Pos2::new(x, y + px(16.0)),
                Vec2::new(w.min(px(300.0)), px(36.0)),
            ),
            s,
        );
    y += px(58.0);
    lbl(ui, x, y, "Environment (KEY=VALUE per line)", s);
    let env_rect = Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(w, px(68.0)));
    kit::bordered(
        &ui.painter().clone(),
        env_rect,
        theme::FIELD,
        theme::LINE,
        px(theme::RADIUS_FIELD),
    );
    ui.new_child(egui::UiBuilder::new().max_rect(env_rect.shrink(px(8.0))))
        .add(
            egui::TextEdit::multiline(&mut step.env)
                .frame(Frame::NONE)
                .text_color(theme::INK)
                .desired_width(f32::INFINITY),
        );
}

fn clean_fields(step: &mut PipelineStep, ui: &mut egui::Ui, x: f32, w: f32, mut y: f32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    lbl(
        ui,
        x,
        y,
        "Targets (folders or patterns, relative to the working dir)",
        s,
    );
    y += px(22.0);
    let mut remove = None;
    for j in 0..step.targets.len() {
        Input::new(&mut step.targets[j])
            .hint("build/ or *.tmp")
            .show(
                ui,
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(w.min(px(280.0)), px(28.0))),
                s,
            );
        if Button::new("X", ButtonKind::Danger)
            .show(
                ui,
                Rect::from_min_size(
                    Pos2::new(x + w.min(px(280.0)) + px(8.0), y),
                    Vec2::new(px(38.0), px(28.0)),
                ),
                s,
            )
            .clicked()
        {
            remove = Some(j);
        }
        y += px(34.0);
    }
    if let Some(j) = remove {
        step.targets.remove(j);
    }
    if Button::new("+ Target", ButtonKind::Secondary)
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(120.0), px(28.0))),
            s,
        )
        .clicked()
    {
        step.targets.push(String::new());
    }
}

fn text(ui: &egui::Ui, x: f32, y: f32, content: &str, s: f32) {
    ui.painter().text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        content,
        FontId::new(theme::px(12.0, s), theme::family_medium()),
        theme::INK_MUTED,
    );
}
