use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, UiBuilder, Vec2};

use super::{Act, Filter};
use crate::features::manifest::{self, DiffEntry, DiffStatus, DiffSummary};
use crate::ui::kit::{self, Input};
use crate::ui::theme;
use crate::validate::human_size;

fn status_color(status: DiffStatus) -> Color32 {
    match status {
        DiffStatus::Added => theme::SUCCESS,
        DiffStatus::Modified => theme::WARNING,
        DiffStatus::Removed => theme::DANGER,
        DiffStatus::Unchanged => theme::INK_MUTED,
    }
}

pub(super) fn progress(ui: &mut egui::Ui, d: Rect, done: usize, total: usize, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let x = d.left() + px(24.0);
    let msg = if total > 0 {
        format!("Hashing {done} / {total} files...")
    } else {
        "Scanning files...".to_string()
    };
    ui.painter().text(
        Pos2::new(x, d.center().y - px(16.0)),
        Align2::LEFT_CENTER,
        msg,
        FontId::new(px(13.0), theme::family_medium()),
        theme::INK_DIM,
    );
    let bar = Rect::from_min_size(
        Pos2::new(x, d.center().y + px(6.0)),
        Vec2::new(d.width() - px(48.0), px(5.0)),
    );
    let frac = if total > 0 {
        done as f32 / total as f32
    } else {
        0.0
    };
    kit::progress_bar(ui, bar, frac, s);
}

pub(super) fn summary_row(ui: &egui::Ui, pos: Pos2, w: f32, summary: &DiffSummary, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let items = [
        (
            "Added",
            summary.added,
            summary.added_bytes,
            DiffStatus::Added,
        ),
        (
            "Modified",
            summary.modified,
            summary.modified_bytes,
            DiffStatus::Modified,
        ),
        (
            "Removed",
            summary.removed,
            summary.removed_bytes,
            DiffStatus::Removed,
        ),
        (
            "Unchanged",
            summary.unchanged,
            summary.unchanged_bytes,
            DiffStatus::Unchanged,
        ),
    ];
    let step = w / 4.0;
    for (i, (label, count, bytes, status)) in items.iter().enumerate() {
        let cx = pos.x + i as f32 * step;
        ui.painter().circle_filled(
            Pos2::new(cx + px(4.0), pos.y + px(2.0)),
            px(4.0),
            status_color(*status),
        );
        ui.painter().text(
            Pos2::new(cx + px(16.0), pos.y),
            Align2::LEFT_CENTER,
            format!("{label}  {count}"),
            FontId::new(px(13.0), theme::family_semibold()),
            theme::INK,
        );
        ui.painter().text(
            Pos2::new(cx + px(16.0), pos.y + px(18.0)),
            Align2::LEFT_CENTER,
            human_size(*bytes),
            FontId::new(px(11.5), theme::family_medium()),
            theme::INK_MUTED,
        );
    }
    ui.painter().text(
        Pos2::new(pos.x + w, pos.y + px(40.0)),
        Align2::RIGHT_CENTER,
        format!(
            "{} changed · ~{} to upload (estimate)",
            summary.changed(),
            human_size(summary.estimated_upload())
        ),
        FontId::new(px(12.5), theme::family_medium()),
        theme::INK_DIM,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn controls(
    ui: &mut egui::Ui,
    x: f32,
    y: f32,
    w: f32,
    search: &mut String,
    filter: Filter,
    sort_size: bool,
    act: &mut Act,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let field = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w * 0.4, px(34.0)));
    Input::new(search).hint("Search files").show(ui, field, s);
    let mut cx = field.right() + px(10.0);
    for (label, f) in [
        ("ALL", Filter::All),
        ("ADD", Filter::Added),
        ("MOD", Filter::Modified),
        ("REM", Filter::Removed),
    ] {
        let size = kit::chip_size(ui, label, s);
        let r = Rect::from_min_size(Pos2::new(cx, y + px(2.0)), size);
        if kit::chip(ui, r, label, filter == f, s).clicked() {
            *act = Act::Filter(f);
        }
        cx += size.x + px(6.0);
    }
    let sort = kit::chip_size(ui, "SIZE", s);
    let r = Rect::from_min_size(Pos2::new(x + w - sort.x, y + px(2.0)), sort);
    if kit::chip(ui, r, "SIZE", sort_size, s).clicked() {
        *act = Act::ToggleSort;
    }
}

pub(super) fn file_list(
    ui: &mut egui::Ui,
    rect: Rect,
    summary: &DiffSummary,
    search: &str,
    filter: Filter,
    sort_size: bool,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let needle = search.to_ascii_lowercase();
    let mut rows: Vec<&DiffEntry> = summary
        .entries
        .iter()
        .filter(|e| match filter {
            Filter::All => true,
            Filter::Added => e.status == DiffStatus::Added,
            Filter::Modified => e.status == DiffStatus::Modified,
            Filter::Removed => e.status == DiffStatus::Removed,
        })
        .filter(|e| needle.is_empty() || e.path.to_ascii_lowercase().contains(&needle))
        .collect();
    if sort_size {
        rows.sort_by_key(|e| std::cmp::Reverse(e.size));
    }

    let mut cui = ui.new_child(UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("review_files")
        .auto_shrink([false, false])
        .show(&mut cui, |ui| {
            ui.set_width(rect.width() - px(14.0));
            let largest = summary.largest(10);
            if !largest.is_empty() && filter == Filter::All && search.is_empty() {
                section(ui, "TOP 10 LARGEST CHANGES", s);
                for entry in largest {
                    row(ui, entry, s);
                }
                section(ui, &format!("ALL FILES  {}", rows.len()), s);
            }
            for entry in rows {
                row(ui, entry, s);
            }
        });
}

fn section(ui: &mut egui::Ui, text: &str, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), px(26.0)), Sense::hover());
    kit::label(
        &ui.painter().clone(),
        Pos2::new(r.left(), r.top() + px(8.0)),
        text,
        s,
    );
}

fn row(ui: &mut egui::Ui, entry: &manifest::DiffEntry, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), px(26.0)), Sense::hover());
    ui.painter().circle_filled(
        Pos2::new(r.left() + px(5.0), r.center().y),
        px(4.0),
        status_color(entry.status),
    );
    ui.painter().text(
        Pos2::new(r.left() + px(18.0), r.center().y),
        Align2::LEFT_CENTER,
        &entry.path,
        FontId::new(px(12.0), theme::family_medium()),
        theme::INK,
    );
    ui.painter().text(
        Pos2::new(r.right() - px(6.0), r.center().y),
        Align2::RIGHT_CENTER,
        human_size(entry.size),
        FontId::new(px(11.5), egui::FontFamily::Monospace),
        theme::INK_MUTED,
    );
}
