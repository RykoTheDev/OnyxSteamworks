use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, UiBuilder, Vec2};

use super::Phase;
use crate::features::verify::VerifyReport;
use crate::ui::kit;
use crate::ui::theme;

pub(super) fn body(ui: &mut egui::Ui, phase: &Phase, done: usize, total: usize, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let d = ui.max_rect();
    let x = d.left() + px(24.0);
    let area = Rect::from_min_max(
        Pos2::new(x, d.top() + px(56.0)),
        Pos2::new(d.right() - px(24.0), d.bottom() - px(80.0)),
    );
    match phase {
        Phase::Idle | Phase::Downloading => status(
            ui,
            area,
            "Downloading the build to a temp folder...",
            theme::INK_DIM,
            s,
        ),
        Phase::Hashing => {
            status(
                ui,
                area,
                &format!("Hashing downloaded files ({done}/{total})..."),
                theme::INK_DIM,
                s,
            );
            let bar = Rect::from_min_size(
                Pos2::new(area.left(), area.top() + px(26.0)),
                Vec2::new(area.width(), px(5.0)),
            );
            kit::progress_bar(ui, bar, done as f32 / total.max(1) as f32, s);
        }
        Phase::Access(msg) => status(ui, area, msg, theme::WARNING, s),
        Phase::Failed(msg) => status(ui, area, msg, theme::DANGER, s),
        Phase::Done(report) => report_body(ui, area, report, s),
    }
}

fn status(ui: &egui::Ui, area: Rect, text: &str, color: Color32, s: f32) {
    let galley = ui.painter().layout(
        text.to_string(),
        FontId::new(theme::px(13.0, s), theme::family_medium()),
        color,
        area.width(),
    );
    ui.painter().galley(area.left_top(), galley, color);
}

fn report_body(ui: &mut egui::Ui, area: Rect, report: &VerifyReport, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let (text, color) = if report.all_match() {
        ("All files match the manifest.".to_string(), theme::SUCCESS)
    } else {
        (
            format!(
                "{} matched · {} missing · {} mismatched · {} extra",
                report.matched,
                report.missing.len(),
                report.mismatched.len(),
                report.extra.len()
            ),
            theme::WARNING,
        )
    };
    ui.painter().text(
        area.left_top(),
        Align2::LEFT_TOP,
        text,
        FontId::new(px(13.5), theme::family_semibold()),
        color,
    );
    let list = Rect::from_min_max(Pos2::new(area.left(), area.top() + px(28.0)), area.max);
    let mut cu = ui.new_child(UiBuilder::new().max_rect(list));
    egui::ScrollArea::vertical()
        .id_salt("verify_report")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            ui.set_width(list.width() - px(14.0));
            for (label, items, c) in [
                ("Missing", &report.missing, theme::DANGER),
                ("Mismatched", &report.mismatched, theme::WARNING),
                ("Extra", &report.extra, theme::INK_MUTED),
            ] {
                if items.is_empty() {
                    continue;
                }
                section(ui, label, s);
                for item in items {
                    let (r, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), px(20.0)),
                        Sense::hover(),
                    );
                    ui.painter().circle_filled(
                        Pos2::new(r.left() + px(4.0), r.center().y),
                        px(3.0),
                        c,
                    );
                    ui.painter().text(
                        Pos2::new(r.left() + px(16.0), r.center().y),
                        Align2::LEFT_CENTER,
                        item,
                        FontId::new(px(12.0), theme::family_medium()),
                        theme::INK_DIM,
                    );
                }
            }
        });
}

fn section(ui: &mut egui::Ui, text: &str, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), px(24.0)), Sense::hover());
    kit::label(
        &ui.painter().clone(),
        Pos2::new(r.left(), r.top() + px(7.0)),
        text,
        s,
    );
}
