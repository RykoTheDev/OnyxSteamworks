use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use super::state;
use crate::app::App;
use crate::model::BuildRecord;
use crate::steamcmd::BuildResult;
use crate::ui::kit;
use crate::ui::theme;

pub(super) fn show(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    s: f32,
    last: Option<&BuildRecord>,
) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    let pad = px(18.0);
    let x = rect.left() + pad;
    let w = rect.width() - pad * 2.0;
    kit::label(
        &ui.painter().clone(),
        Pos2::new(x, rect.top() + pad),
        "STAGES",
        s,
    );

    let seg = Rect::from_min_size(Pos2::new(x, rect.top() + px(48.0)), Vec2::new(w, px(6.0)));
    kit::segment_progress(
        ui,
        seg,
        state::STAGES.len(),
        app.build_ui.filled_stages(),
        s,
    );

    let elapsed = app.build_ui.elapsed();
    let secs = elapsed.as_secs();
    ui.painter().text(
        Pos2::new(x, seg.bottom() + px(22.0)),
        Align2::LEFT_CENTER,
        app.build_ui.stage_name(),
        FontId::new(px(14.0), theme::family_semibold()),
        theme::INK,
    );
    ui.painter().text(
        Pos2::new(x + w, seg.bottom() + px(22.0)),
        Align2::RIGHT_CENTER,
        format!("{:02}:{:02}", secs / 60, secs % 60),
        FontId::new(px(13.0), egui::FontFamily::Monospace),
        theme::INK_DIM,
    );

    let bar = Rect::from_min_size(Pos2::new(x, seg.bottom() + px(40.0)), Vec2::new(w, px(5.0)));
    kit::progress_bar(ui, bar, app.build_ui.progress, s);

    if app.build_ui.speed.has_data() {
        speed_widget(app, ui, Pos2::new(x, bar.bottom() + px(14.0)), w, s);
    } else if let Some(outcome) = &app.build_ui.outcome {
        let (text, color) = match outcome {
            BuildResult::Success { app_id } => (format!("Success  app {app_id}"), theme::SUCCESS),
            BuildResult::Failure { .. } => ("Failed".to_string(), theme::DANGER),
            BuildResult::Cancelled => ("Cancelled".to_string(), theme::WARNING),
        };
        ui.painter().text(
            Pos2::new(x, bar.bottom() + px(22.0)),
            Align2::LEFT_CENTER,
            text,
            FontId::new(px(12.5), theme::family_medium()),
            color,
        );
    }

    verify_offer(
        app,
        ui,
        Rect::from_min_size(
            Pos2::new(x, bar.bottom() + px(42.0)),
            Vec2::new(w, px(36.0)),
        ),
        s,
    );

    if let Some(record) = last {
        use crate::ui::pages::history;
        let y = rect.bottom() - px(22.0);
        let d = record.duration_ms / 1000;
        let summary = format!(
            "{}  ·  {:02}:{:02}  ·  {}",
            history::status_label(record.status),
            d / 60,
            d % 60,
            history::rel_time(record.timestamp)
        );
        ui.painter().text(
            Pos2::new(x, y),
            Align2::LEFT_CENTER,
            "LAST BUILD",
            FontId::new(px(11.0), theme::family_label()),
            theme::INK_MUTED,
        );
        ui.painter().text(
            Pos2::new(x + w, y),
            Align2::RIGHT_CENTER,
            summary,
            FontId::new(px(12.0), theme::family_medium()),
            history::status_color(record.status),
        );
    }
}

fn verify_offer(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let ready = app.build_ui.verify
        && !app.build_ui.verify_ui.running()
        && matches!(&app.build_ui.outcome, Some(BuildResult::Success { app_id }) if !app_id.is_empty());
    if !ready {
        return;
    }
    if kit::Button::new("VERIFY BUILD", kit::ButtonKind::Secondary)
        .icon(theme::icon::CHECK)
        .show(ui, rect, s)
        .clicked()
    {
        let ctx = ui.ctx().clone();
        let project = app.projects.get(app.build_ui.project_sel).cloned();
        let record = project
            .as_ref()
            .and_then(|p| app.history_ui.latest_for(&p.id).cloned());
        if let (Some(project), Some(record)) = (project, record) {
            super::verify::begin(app, &project, record, &ctx);
        }
    }
}

fn speed_widget(app: &App, ui: &mut egui::Ui, pos: Pos2, w: f32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    ui.painter().text(
        pos,
        Align2::LEFT_TOP,
        "UPLOAD SPEED",
        FontId::new(px(11.0), theme::family_label()),
        theme::INK_MUTED,
    );
    let eta = app
        .build_ui
        .speed
        .eta_secs()
        .map(|t| format!("ETA {:02}:{:02}", t / 60, t % 60))
        .unwrap_or_default();
    ui.painter().text(
        Pos2::new(pos.x + w, pos.y),
        Align2::RIGHT_TOP,
        eta,
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK_DIM,
    );
    let chart = Rect::from_min_size(Pos2::new(pos.x, pos.y + px(20.0)), Vec2::new(w, px(32.0)));
    kit::line_chart(ui, chart, app.build_ui.speed.samples(), theme::INK, s);
}
