use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, UiBuilder, Vec2};

use crate::app::App;
use crate::features::queue::JobStatus;
use crate::ui::anim;
use crate::ui::kit;
use crate::ui::theme;

fn arrow_button(ui: &mut egui::Ui, rect: Rect, glyph: &str, s: f32) -> egui::Response {
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id(("arrow", glyph, rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    let ctx = ui.ctx().clone();
    let hover = anim::anim_bool(&ctx, id.with("h"), resp.hovered(), 0.12, anim::ease_out_cubic);
    let painter = ui.painter();
    painter.rect_filled(rect, theme::corner(px(theme::RADIUS_FIELD)), theme::lerp_color(theme::SURFACE, theme::FIELD, hover));
    painter.text(rect.center(), Align2::CENTER_CENTER, glyph, FontId::new(px(14.0), theme::family_semibold()), theme::lerp_color(theme::INK_MUTED, theme::INK, hover));
    resp
}

fn status_color(status: JobStatus) -> egui::Color32 {
    match status {
        JobStatus::Succeeded => theme::SUCCESS,
        JobStatus::Failed => theme::DANGER,
        JobStatus::Retrying => theme::WARNING,
        JobStatus::Running => theme::INK,
        _ => theme::INK_MUTED,
    }
}

pub(super) fn cards(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let mut up = None;
    let mut down = None;
    let mut next = None;
    let mut remove = None;
    let mut cu = ui.new_child(UiBuilder::new().max_rect(rect));
    cu.set_clip_rect(rect);
    egui::ScrollArea::vertical().id_salt("queue_list").auto_shrink([false, false]).show(&mut cu, |ui| {
        ui.set_width(rect.width() - px(14.0));
        for (i, job) in app.queue.jobs.iter().enumerate() {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), px(66.0)), Sense::hover());
            let ctx = ui.ctx().clone();
            let hover = anim::anim_bool(&ctx, ui.make_persistent_id(("jobcard", &job.id)), resp.hovered(), 0.12, anim::ease_out_cubic);
            kit::bordered(ui.painter(), r, theme::lerp_color(theme::SURFACE_2, theme::FIELD, hover * 0.6), theme::LINE, px(theme::RADIUS_CARD));
            ui.painter().circle_filled(Pos2::new(r.left() + px(16.0), r.top() + px(20.0)), px(4.0), status_color(job.status));
            ui.painter().text(Pos2::new(r.left() + px(28.0), r.top() + px(16.0)), Align2::LEFT_CENTER, job.project_name(), FontId::new(px(13.5), theme::family_semibold()), theme::INK);
            let branch = if job.branch.is_empty() { "default".to_string() } else { job.branch.clone() };
            let mut meta = format!("{}  ·  {branch}", job.status.label());
            if job.options.preview {
                meta.push_str("  ·  preview");
            }
            if job.attempts > 0 {
                meta.push_str(&format!("  ·  try {}/{}", job.attempts, job.max_retries));
            }
            ui.painter().text(Pos2::new(r.left() + px(28.0), r.top() + px(38.0)), Align2::LEFT_CENTER, meta, FontId::new(px(11.5), theme::family_medium()), theme::INK_MUTED);
            if job.status == JobStatus::Retrying
                && let Some(secs) = job.countdown_secs()
            {
                ui.painter().text(Pos2::new(r.left() + px(28.0), r.top() + px(54.0)), Align2::LEFT_CENTER, format!("retrying in {}:{:02}", secs / 60, secs % 60), FontId::new(px(11.5), theme::family_semibold()), theme::WARNING);
            } else if !job.message.is_empty() {
                ui.painter().text(Pos2::new(r.left() + px(28.0), r.top() + px(54.0)), Align2::LEFT_CENTER, &job.message, FontId::new(px(11.0), theme::family_medium()), theme::INK_DIM);
            }

            let bw = px(30.0);
            let dz = Rect::from_min_size(Pos2::new(r.right() - px(12.0) - bw, r.center().y - bw / 2.0), Vec2::splat(bw));
            let nz = Rect::from_min_size(Pos2::new(dz.left() - px(6.0) - bw, r.center().y - bw / 2.0), Vec2::splat(bw));
            let dnz = Rect::from_min_size(Pos2::new(nz.left() - px(6.0) - bw, r.center().y - bw / 2.0), Vec2::splat(bw));
            let uz = Rect::from_min_size(Pos2::new(dnz.left() - px(6.0) - bw, r.center().y - bw / 2.0), Vec2::splat(bw));
            if arrow_button(ui, uz, "\u{2191}", s).clicked() {
                up = Some(i);
            }
            if arrow_button(ui, dnz, "\u{2193}", s).clicked() {
                down = Some(i);
            }
            if kit::icon_button(ui, nz, theme::icon::PLAY, s).clicked() {
                next = Some(i);
            }
            if kit::icon_button(ui, dz, theme::icon::TRASH, s).clicked() {
                remove = Some(i);
            }
            ui.add_space(px(10.0));
        }
    });
    if let Some(i) = up {
        app.queue.move_job(i, true);
    }
    if let Some(i) = down {
        app.queue.move_job(i, false);
    }
    if let Some(i) = next {
        app.queue.run_next(i);
    }
    if let Some(i) = remove {
        app.queue.remove_job(i);
    }
}
