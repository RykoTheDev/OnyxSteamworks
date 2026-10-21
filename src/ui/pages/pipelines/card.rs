use eframe::egui::{self, Align2, FontId, Frame, Pos2, Rect, Vec2};

use crate::model::{PipelineStep, StepKind};
use crate::ui::kit;
use crate::ui::kit::{Button, ButtonKind, Input};
use crate::ui::theme;

pub(super) fn extra_height(step: &PipelineStep) -> f32 {
    let mut h = 58.0 + 48.0;
    match step.kind {
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

pub(super) fn fields(step: &mut PipelineStep, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let x = rect.left();
    let w = rect.width();
    let mut y = rect.top();

    lbl(ui, x, y, "STEP NAME", s);
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
    ui.put(
        to,
        egui::DragValue::new(&mut step.timeout_secs)
            .range(0..=86_400)
            .suffix(" s timeout"),
    );
    y += px(44.0);

    match step.kind {
        StepKind::RunCommand => run_command_fields(step, ui, x, w, y, s),
        StepKind::Clean => clean_fields(step, ui, x, w, y, s),
        StepKind::Notify | StepKind::Confirm => {
            lbl(ui, x, y, "MESSAGE", s);
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
            "Generates the VDFs and runs the Steam upload (non-interactive).",
            s,
        ),
        StepKind::Backup => text(ui, x, y + px(6.0), "Backup is a placeholder until Q5.", s),
    }
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
    lbl(ui, x, y, "EXECUTABLE", s);
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
    lbl(ui, x, y, "ARGUMENTS", s);
    Input::new(&mut step.args)
        .hint("--headless --path . --export-release ...")
        .show(
            ui,
            Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(w, px(36.0))),
            s,
        );
    y += px(58.0);
    lbl(ui, x, y, "WORKING DIR", s);
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
    lbl(ui, x, y, "ENV (KEY=VALUE PER LINE)", s);
    let env_rect = Rect::from_min_size(Pos2::new(x, y + px(16.0)), Vec2::new(w, px(68.0)));
    kit::bordered(
        &ui.painter().clone(),
        env_rect,
        theme::FIELD,
        theme::LINE,
        px(theme::RADIUS_FIELD),
    );
    ui.put(
        env_rect.shrink(px(8.0)),
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
        "TARGETS (FOLDERS OR PATTERNS, RELATIVE TO WORKING DIR)",
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
    if Button::new("+ TARGET", ButtonKind::Secondary)
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
