use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use crate::app::App;
use crate::features::advisor::{self, Analytics, BuildPoint};
use crate::ui::kit;
use crate::ui::theme;
use crate::validate::human_size;

mod advisor_view;

const KEEP: usize = 20;

pub struct AnalyticsState {
    pub project_sel: usize,
    cache: HashMap<String, Analytics>,
    loading: HashSet<String>,
    tx: Sender<(String, Analytics)>,
    rx: Receiver<(String, Analytics)>,
}

impl AnalyticsState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            project_sel: 0,
            cache: HashMap::new(),
            loading: HashSet::new(),
            tx,
            rx,
        }
    }
}

fn settings() -> crate::config::Settings {
    crate::config::load_settings().unwrap_or_default()
}

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    while let Ok((key, data)) = app.analytics_ui.rx.try_recv() {
        app.analytics_ui.cache.insert(key.clone(), data);
        app.analytics_ui.loading.remove(&key);
    }
    app.history_ui.ensure_loaded();

    if app.projects.is_empty() {
        kit::empty_state(
            ui,
            rect,
            theme::icon::HISTORY,
            "No project yet",
            "Create a project and run builds to see analytics.",
            s,
        );
        return;
    }
    if app.analytics_ui.project_sel >= app.projects.len() {
        app.analytics_ui.project_sel = 0;
    }
    project_chips(app, ui, rect, rect.top(), s);

    let project = app.projects[app.analytics_ui.project_sel].clone();
    let records: Vec<crate::model::BuildRecord> = app
        .history_ui
        .for_project(&project.id)
        .into_iter()
        .cloned()
        .collect();
    let latest_ts = records.first().map(|r| r.timestamp).unwrap_or(0);
    let key = format!("{}|{}|{}", project.id, records.len(), latest_ts);

    let cfg = settings();
    if !app.analytics_ui.cache.contains_key(&key) && !app.analytics_ui.loading.contains(&key) {
        app.analytics_ui.loading.insert(key.clone());
        let tx = app.analytics_ui.tx.clone();
        let ctx2 = ctx.clone();
        let large_bytes = cfg.advisor_large_mb.max(1) as u64 * 1024 * 1024;
        let key_for_thread = key.clone();
        std::thread::spawn(move || {
            let data = advisor::run(&project, &records, large_bytes, KEEP);
            let _ = tx.send((key_for_thread, data));
            ctx2.request_repaint();
        });
    }

    let body = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + px(40.0)), rect.max);
    match app.analytics_ui.cache.get(&key).cloned() {
        Some(data) if data.total_builds == 0 => {
            kit::empty_state(
                ui,
                body,
                theme::icon::HISTORY,
                "No builds yet",
                "Run a build to start collecting analytics.",
                s,
            );
        }
        Some(data) => render(ui, body, &data, &cfg, s),
        None => {
            let c = body.center();
            kit::progress_ring(ui, c, px(16.0), 0.25, theme::INK_DIM, s);
            ui.painter().text(
                Pos2::new(c.x, c.y + px(30.0)),
                Align2::CENTER_CENTER,
                "Analyzing builds...",
                FontId::new(px(12.5), theme::family_medium()),
                theme::INK_MUTED,
            );
            ctx.request_repaint();
        }
    }
}

fn project_chips(app: &mut App, ui: &mut egui::Ui, rect: Rect, y: f32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let mut x = rect.left();
    for i in 0..app.projects.len() {
        let label = if app.projects[i].name.is_empty() {
            "Untitled".to_string()
        } else {
            app.projects[i].name.clone()
        };
        let size = kit::chip_size(ui, &label, s);
        if x + size.x > rect.right() {
            break;
        }
        if kit::chip(
            ui,
            Rect::from_min_size(Pos2::new(x, y), size),
            &label,
            i == app.analytics_ui.project_sel,
            s,
        )
        .clicked()
        {
            app.analytics_ui.project_sel = i;
        }
        x += size.x + px(8.0);
    }
}

fn render(ui: &mut egui::Ui, rect: Rect, data: &Analytics, cfg: &crate::config::Settings, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let mut top = rect.top();

    let threshold = cfg.analytics_growth_pct.max(1) as f32 / 100.0;
    if let Some(g) = data.growth
        && g >= threshold
    {
        banner(
            ui,
            Rect::from_min_size(
                Pos2::new(rect.left(), top),
                Vec2::new(rect.width(), px(30.0)),
            ),
            g,
            s,
        );
        top += px(38.0);
    }

    let gap = px(14.0);
    let col_w = (rect.width() - gap * 2.0) / 3.0;
    let ch = px(150.0);
    let sizes: Vec<f32> = data.points.iter().map(|p| p.size as f32).collect();
    let durs: Vec<f32> = data
        .points
        .iter()
        .map(|p| p.duration_ms as f32 / 1000.0)
        .collect();
    chart_panel(
        ui,
        Rect::from_min_size(Pos2::new(rect.left(), top), Vec2::new(col_w, ch)),
        "Build size",
        &sizes,
        data.points
            .last()
            .map(|p| human_size(p.size))
            .unwrap_or_default(),
        s,
    );
    chart_panel(
        ui,
        Rect::from_min_size(
            Pos2::new(rect.left() + col_w + gap, top),
            Vec2::new(col_w, ch),
        ),
        "Duration",
        &durs,
        data.points
            .last()
            .map(|p| super::fmt_duration(p.duration_ms))
            .unwrap_or_default(),
        s,
    );
    success_panel(
        ui,
        Rect::from_min_size(
            Pos2::new(rect.left() + (col_w + gap) * 2.0, top),
            Vec2::new(col_w, ch),
        ),
        data,
        s,
    );
    top += ch + px(18.0);

    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left(), top),
        "Last 10 builds",
        s,
    );
    top += px(24.0);
    last_builds(
        ui,
        Rect::from_min_size(
            Pos2::new(rect.left(), top),
            Vec2::new(rect.width(), px(64.0)),
        ),
        &data.points,
        s,
    );
    top += px(80.0);

    advisor_view::show(
        ui,
        Rect::from_min_max(Pos2::new(rect.left(), top), rect.max),
        &data.report,
        s,
    );
}

fn banner(ui: &mut egui::Ui, rect: Rect, growth: f32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::SURFACE_2,
        theme::WARNING,
        px(theme::RADIUS_FIELD),
    );
    ui.painter().circle_filled(
        Pos2::new(rect.left() + px(16.0), rect.center().y),
        px(4.0),
        theme::WARNING,
    );
    let msg = format!(
        "Latest build is {:.0}% larger than the previous one.",
        growth * 100.0
    );
    ui.painter().text(
        Pos2::new(rect.left() + px(30.0), rect.center().y),
        Align2::LEFT_CENTER,
        msg,
        FontId::new(px(12.5), theme::family_semibold()),
        theme::INK,
    );
}

fn chart_panel(
    ui: &mut egui::Ui,
    rect: Rect,
    title: &str,
    samples: &[f32],
    latest: String,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left() + px(16.0), rect.top() + px(14.0)),
        title,
        s,
    );
    ui.painter().text(
        Pos2::new(rect.right() - px(16.0), rect.top() + px(20.0)),
        Align2::RIGHT_CENTER,
        latest,
        FontId::new(px(14.0), theme::family_semibold()),
        theme::INK,
    );
    let plot = Rect::from_min_max(
        Pos2::new(rect.left() + px(16.0), rect.top() + px(48.0)),
        Pos2::new(rect.right() - px(16.0), rect.bottom() - px(18.0)),
    );
    if samples.len() < 2 {
        ui.painter().text(
            plot.center(),
            Align2::CENTER_CENTER,
            "Need 2+ builds",
            FontId::new(px(11.5), theme::family_medium()),
            theme::INK_MUTED,
        );
        return;
    }
    let max = samples.iter().copied().fold(f32::MIN, f32::max).max(1e-6);
    ui.painter().line_segment(
        [
            Pos2::new(plot.left(), plot.bottom()),
            Pos2::new(plot.right(), plot.bottom()),
        ],
        egui::Stroke::new(1.0, theme::LINE),
    );
    kit::line_chart(ui, plot, samples, theme::INK, s);
    let n = samples.len();
    let last = samples[n - 1];
    let cx = plot.left() + plot.width();
    let cy = plot.bottom() - plot.height() * (last / max).clamp(0.0, 1.0);
    ui.painter()
        .circle_filled(Pos2::new(cx, cy), px(3.0), theme::INK);
}

fn success_panel(ui: &mut egui::Ui, rect: Rect, data: &Analytics, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left() + px(16.0), rect.top() + px(14.0)),
        "Success rate",
        s,
    );
    let center = Pos2::new(rect.center().x, rect.center().y + px(8.0));
    let color = if data.success_rate >= 0.8 {
        theme::SUCCESS
    } else if data.success_rate >= 0.5 {
        theme::WARNING
    } else {
        theme::DANGER
    };
    kit::progress_ring(ui, center, px(34.0), data.success_rate, color, s);
    ui.painter().text(
        center,
        Align2::CENTER_CENTER,
        format!("{:.0}%", data.success_rate * 100.0),
        FontId::new(px(18.0), theme::family_semibold()),
        theme::INK,
    );
    ui.painter().text(
        Pos2::new(center.x, rect.bottom() - px(16.0)),
        Align2::CENTER_CENTER,
        format!("{} builds", data.total_builds),
        FontId::new(px(11.5), theme::family_medium()),
        theme::INK_MUTED,
    );
}

fn last_builds(ui: &mut egui::Ui, rect: Rect, points: &[BuildPoint], s: f32) {
    let px = |v: f32| theme::px(v, s);
    let recent: Vec<&BuildPoint> = points.iter().rev().take(10).collect();
    if recent.is_empty() {
        ui.painter().text(
            Pos2::new(rect.left(), rect.top() + px(10.0)),
            Align2::LEFT_TOP,
            "No builds with manifests yet.",
            FontId::new(px(12.5), theme::family_medium()),
            theme::INK_MUTED,
        );
        return;
    }
    let n = recent.len();
    let gap = px(8.0);
    let cw = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
    for (i, p) in recent.iter().enumerate() {
        let x = rect.left() + i as f32 * (cw + gap);
        let cr = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(cw, rect.height()));
        kit::bordered(
            ui.painter(),
            cr,
            theme::SURFACE_2,
            theme::LINE,
            px(theme::RADIUS_CARD),
        );
        let color = if p.success {
            theme::SUCCESS
        } else {
            theme::DANGER
        };
        ui.painter().circle_filled(
            Pos2::new(cr.left() + px(12.0), cr.top() + px(14.0)),
            px(4.0),
            color,
        );
        ui.painter().text(
            Pos2::new(cr.left() + px(12.0), cr.center().y + px(2.0)),
            Align2::LEFT_CENTER,
            human_size(p.size),
            FontId::new(px(12.5), theme::family_semibold()),
            theme::INK,
        );
        ui.painter().text(
            Pos2::new(cr.left() + px(12.0), cr.bottom() - px(12.0)),
            Align2::LEFT_CENTER,
            super::fmt_duration(p.duration_ms),
            FontId::new(px(11.0), theme::family_medium()),
            theme::INK_MUTED,
        );
    }
}
