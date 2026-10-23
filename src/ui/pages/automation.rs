mod card;
mod pipelines;
mod runs;
mod scheduler;
mod schedules;
mod state;
mod timeline;

pub use runs::RunsState;
pub use scheduler::SchedulerState;
pub use state::PipelineState;

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Vec2};

use crate::app::{App, Page};
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::layout::Layout;
use crate::ui::pages::history::rel_time;
use crate::ui::{nav, pages, theme};

pub fn show(app: &mut App, ui: &mut egui::Ui, layout: &Layout) {
    let s = layout.scale;
    let ctx = ui.ctx().clone();
    let idx = nav::index(Page::Automation);
    let tabs = nav::ITEMS[idx].sub_tabs;
    let mut active = app.sub_tabs[idx];
    let body = pages::hero_and_tabs(ui, layout.content, s, "Automation", tabs, &mut active);
    app.sub_tabs[idx] = active;
    let body = approvals_banner(app, ui, body, s);

    match active {
        1 => schedules::show(app, ui, body, s),
        2 => runs::show(app, ui, body, s),
        _ => pipelines::show(app, ui, body, s, &ctx),
    }
}

fn approvals_banner(app: &mut App, ui: &mut egui::Ui, body: Rect, s: f32) -> Rect {
    let px = |v: f32| theme::px(v, s);
    let pending = app.scheduler.approvals.clone();
    if pending.is_empty() {
        return body;
    }
    let mut y = body.top();
    let mut decision = None;
    for a in &pending {
        let row = Rect::from_min_size(Pos2::new(body.left(), y), Vec2::new(body.width(), px(52.0)));
        kit::bordered(
            &ui.painter().clone(),
            row,
            theme::with_alpha(theme::WARNING, 18),
            theme::with_alpha(theme::WARNING, 120),
            px(theme::RADIUS_FIELD),
        );
        ui.painter().text(
            Pos2::new(row.left() + px(16.0), row.top() + px(17.0)),
            Align2::LEFT_CENTER,
            format!(
                "{} for {} is waiting for approval",
                a.pipeline_name, a.project_name
            ),
            FontId::new(px(13.0), theme::family_semibold()),
            theme::INK,
        );
        let detail = if a.message.is_empty() {
            format!("Requested {} by a background run", rel_time(a.requested_ms))
        } else {
            format!("{}  ·  requested {}", a.message, rel_time(a.requested_ms))
        };
        ui.painter().text(
            Pos2::new(row.left() + px(16.0), row.top() + px(36.0)),
            Align2::LEFT_CENTER,
            detail,
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
        let approve = Rect::from_min_size(
            Pos2::new(row.right() - px(118.0), row.top() + px(9.0)),
            Vec2::new(px(108.0), px(34.0)),
        );
        if Button::new("Approve", ButtonKind::Primary)
            .icon(theme::icon::CHECK)
            .show(ui, approve, s)
            .clicked()
        {
            decision = Some((a.id.clone(), true));
        }
        if Button::new("Reject", ButtonKind::Danger)
            .show(
                ui,
                Rect::from_min_size(
                    Pos2::new(approve.left() - px(98.0), approve.top()),
                    Vec2::new(px(88.0), px(34.0)),
                ),
                s,
            )
            .clicked()
        {
            decision = Some((a.id.clone(), false));
        }
        y = row.bottom() + px(8.0);
    }
    if let Some((id, approve)) = decision {
        app.scheduler.decide(&id, approve);
        app.toasts.push(
            kit::ToastLevel::Success,
            if approve { "Approved" } else { "Rejected" },
        );
    }
    Rect::from_min_max(Pos2::new(body.left(), y + px(6.0)), body.max)
}
