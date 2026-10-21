use eframe::egui::{self, Align2, FontId, Pos2, Rect, UiBuilder, Vec2};

use crate::features::advisor::AdvisorReport;
use crate::ui::kit;
use crate::ui::theme;
use crate::validate::human_size;

pub fn show(ui: &mut egui::Ui, rect: Rect, report: &AdvisorReport, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left(), rect.top()),
        "PATCH-SIZE ADVISOR",
        s,
    );
    let body = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + px(24.0)), rect.max);

    if report.advice.is_empty()
        && report.large_churn.is_empty()
        && report.entropy.is_empty()
        && report.duplicates.is_empty()
    {
        ui.painter().text(
            Pos2::new(body.left(), body.top() + px(6.0)),
            Align2::LEFT_TOP,
            "Nothing to flag. Build sizes look healthy.",
            FontId::new(px(12.5), theme::family_medium()),
            theme::INK_MUTED,
        );
        return;
    }

    let mut cu = ui.new_child(UiBuilder::new().max_rect(body));
    cu.set_clip_rect(body);
    egui::ScrollArea::vertical()
        .id_salt("advisor_scroll")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            ui.set_width(body.width() - px(14.0));
            for line in &report.advice {
                advice_row(ui, line, s);
            }
            if !report.large_churn.is_empty() {
                section(ui, "LARGE FILES CHANGING OFTEN", s);
                for c in &report.large_churn {
                    finding(
                        ui,
                        &c.path,
                        &format!(
                            "depot {}  ·  {}  ·  changed {}/{} builds",
                            c.depot_id,
                            human_size(c.size),
                            c.changes,
                            c.builds
                        ),
                        theme::WARNING,
                        s,
                    );
                }
            }
            if !report.entropy.is_empty() {
                section(ui, "LIKELY COMPRESSED OR ENCRYPTED", s);
                for e in &report.entropy {
                    finding(
                        ui,
                        &e.path,
                        &format!(
                            "depot {}  ·  {}  ·  entropy {:.2}",
                            e.depot_id,
                            human_size(e.size),
                            e.entropy
                        ),
                        theme::INK_DIM,
                        s,
                    );
                }
            }
            if !report.duplicates.is_empty() {
                section(
                    ui,
                    &format!(
                        "DUPLICATES ACROSS DEPOTS  ·  {} wasted",
                        human_size(report.wasted_total)
                    ),
                    s,
                );
                for d in &report.duplicates {
                    let first = d.paths.first().cloned().unwrap_or_default();
                    finding(
                        ui,
                        &first,
                        &format!(
                            "{} each  ·  {} copies  ·  {} wasted",
                            human_size(d.size),
                            d.count,
                            human_size(d.wasted)
                        ),
                        theme::DANGER,
                        s,
                    );
                }
            }
            ui.add_space(px(8.0));
        });
}

fn advice_row(ui: &mut egui::Ui, text: &str, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let galley = ui.painter().layout(
        text.to_string(),
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_DIM,
        ui.available_width() - px(28.0),
    );
    let h = galley.size().y + px(20.0);
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), egui::Sense::hover());
    kit::bordered(
        ui.painter(),
        r,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_CARD),
    );
    ui.painter().text(
        Pos2::new(r.left() + px(14.0), r.center().y),
        Align2::LEFT_CENTER,
        "!",
        FontId::new(px(14.0), theme::family_semibold()),
        theme::WARNING,
    );
    ui.painter().galley(
        Pos2::new(r.left() + px(28.0), r.top() + px(10.0)),
        galley,
        theme::INK_DIM,
    );
    ui.add_space(px(8.0));
}

fn section(ui: &mut egui::Ui, title: &str, s: f32) {
    let px = |v: f32| theme::px(v, s);
    ui.add_space(px(6.0));
    let (r, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), px(22.0)),
        egui::Sense::hover(),
    );
    kit::label(
        ui.painter(),
        Pos2::new(r.left(), r.top() + px(4.0)),
        title,
        s,
    );
}

fn finding(ui: &mut egui::Ui, path: &str, meta: &str, dot: egui::Color32, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let (r, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), px(40.0)),
        egui::Sense::hover(),
    );
    kit::bordered(
        ui.painter(),
        r,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_CARD),
    );
    ui.painter()
        .circle_filled(Pos2::new(r.left() + px(14.0), r.center().y), px(4.0), dot);
    let shown = ellipsize(path, 56);
    ui.painter().text(
        Pos2::new(r.left() + px(28.0), r.center().y - px(7.0)),
        Align2::LEFT_CENTER,
        shown,
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK,
    );
    ui.painter().text(
        Pos2::new(r.left() + px(28.0), r.center().y + px(8.0)),
        Align2::LEFT_CENTER,
        meta,
        FontId::new(px(11.0), theme::family_medium()),
        theme::INK_MUTED,
    );
    ui.add_space(px(8.0));
}

fn ellipsize(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_string();
    }
    let tail: String = text.chars().skip(count - max + 1).collect();
    format!("…{tail}")
}
