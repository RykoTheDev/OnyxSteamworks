mod detail;
mod state;
mod table;

pub use state::{HistoryState, RangeFilter, StatusFilter};

use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::app::{App, Page};
use crate::model::BuildStatus;
use crate::ui::layout::Layout;
use crate::ui::{kit, nav, pages, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let s = layout.scale;
    let idx = nav::index(Page::History);
    let tabs = nav::ITEMS[idx].sub_tabs;
    let mut active = app.sub_tabs[idx];
    let body = pages::hero_and_tabs(ui, layout.content, s, "HISTORY", tabs, &mut active);
    app.sub_tabs[idx] = active;

    match active {
        0 => builds_tab(app, ui, body, s),
        1 => kit::empty_state(
            ui,
            body,
            theme::icon::PACKAGE,
            "No backups yet",
            "Build backups will appear here.",
            s,
        ),
        2 => kit::empty_state(
            ui,
            body,
            theme::icon::SPARKLE,
            "No patch notes yet",
            "Release notes will appear here.",
            s,
        ),
        _ => kit::empty_state(
            ui,
            body,
            theme::icon::HISTORY,
            "No analytics yet",
            "Build analytics will appear here.",
            s,
        ),
    }

    let ctx = ui.ctx().clone();
    detail::show(app, ui, s, &ctx);
}

fn builds_tab(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    app.history_ui.ensure_loaded();

    let project_ids: Vec<String> = app.projects.iter().map(|p| p.id.clone()).collect();
    let mut y = project_chips(app, ui, rect, rect.top(), s);
    y = status_range_chips(app, ui, rect, y + px(10.0), s);

    let table_rect = Rect::from_min_max(Pos2::new(rect.left(), y + px(12.0)), rect.max);
    kit::bordered(
        &ui.painter().clone(),
        table_rect,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_PANEL_MIN),
    );
    let inner = table_rect.shrink(px(14.0));

    let refs = app.history_ui.filtered(&project_ids);
    let clicked = table::show(ui, inner, &refs, &app.projects, s);
    let picked = clicked.and_then(|i| refs.get(i).map(|r| (*r).clone()));
    if let Some(rec) = picked {
        app.history_ui.open_detail(rec);
    }
}

fn project_chips(app: &mut App, ui: &mut egui::Ui, rect: Rect, top: f32, s: f32) -> f32 {
    let px = |v: f32| theme::px(v, s);
    let mut labels = vec!["All".to_string()];
    labels.extend(app.projects.iter().map(|p| {
        if p.name.is_empty() {
            "Untitled".to_string()
        } else {
            p.name.clone()
        }
    }));
    let mut x = rect.left();
    let mut y = top;
    for (i, label) in labels.iter().enumerate() {
        let size = kit::chip_size(ui, label, s);
        if x + size.x > rect.right() {
            x = rect.left();
            y += size.y + px(8.0);
        }
        let r = Rect::from_min_size(Pos2::new(x, y), size);
        if kit::chip(ui, r, label, app.history_ui.project_filter == i, s).clicked() {
            app.history_ui.project_filter = i;
        }
        x += size.x + px(8.0);
    }
    y + px(30.0)
}

fn status_range_chips(app: &mut App, ui: &mut egui::Ui, rect: Rect, top: f32, s: f32) -> f32 {
    let px = |v: f32| theme::px(v, s);
    let statuses = [
        (StatusFilter::All, "All"),
        (StatusFilter::Success, "Success"),
        (StatusFilter::Failed, "Failed"),
        (StatusFilter::Cancelled, "Cancelled"),
    ];
    let mut x = rect.left();
    for (val, label) in statuses {
        let size = kit::chip_size(ui, label, s);
        let r = Rect::from_min_size(Pos2::new(x, top), size);
        if kit::chip(ui, r, label, app.history_ui.status_filter == val, s).clicked() {
            app.history_ui.status_filter = val;
        }
        x += size.x + px(8.0);
    }
    x += px(24.0);
    let ranges = [
        (RangeFilter::All, "All time"),
        (RangeFilter::Day, "24h"),
        (RangeFilter::Week, "7d"),
        (RangeFilter::Month, "30d"),
    ];
    for (val, label) in ranges {
        let size = kit::chip_size(ui, label, s);
        let r = Rect::from_min_size(Pos2::new(x, top), size);
        if kit::chip(ui, r, label, app.history_ui.range_filter == val, s).clicked() {
            app.history_ui.range_filter = val;
        }
        x += size.x + px(8.0);
    }
    top + px(30.0)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(super) fn rel_time(ts: i64) -> String {
    let sec = (now_ms() - ts).max(0) / 1000;
    if sec < 60 {
        "just now".to_string()
    } else if sec < 3600 {
        format!("{}m ago", sec / 60)
    } else if sec < 86_400 {
        format!("{}h ago", sec / 3600)
    } else if sec < 7 * 86_400 {
        format!("{}d ago", sec / 86_400)
    } else {
        format!("{}w ago", sec / (7 * 86_400))
    }
}

pub(super) fn fmt_duration(ms: u64) -> String {
    let sec = ms / 1000;
    format!("{:02}:{:02}", sec / 60, sec % 60)
}

pub(super) fn status_label(status: BuildStatus) -> &'static str {
    match status {
        BuildStatus::Pending => "Pending",
        BuildStatus::Running => "Running",
        BuildStatus::Success => "Success",
        BuildStatus::Failed => "Failed",
        BuildStatus::Cancelled => "Cancelled",
    }
}

pub(super) fn status_color(status: BuildStatus) -> Color32 {
    match status {
        BuildStatus::Success => theme::SUCCESS,
        BuildStatus::Failed => theme::DANGER,
        BuildStatus::Cancelled => theme::WARNING,
        _ => theme::INK_MUTED,
    }
}
