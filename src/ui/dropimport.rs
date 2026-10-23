use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Align2, Color32, FontId, Order, Pos2, Rect, Stroke, Vec2};

use crate::app::{App, Page};
use crate::features::importer::{self, Detected};
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

pub struct DropState {
    hovering: bool,
    pending: Option<Detected>,
    busy: bool,
    tx: Sender<Detected>,
    rx: Receiver<Detected>,
}

impl DropState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            hovering: false,
            pending: None,
            busy: false,
            tx,
            rx,
        }
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, frame: Rect, s: f32) {
    let ctx = ui.ctx().clone();
    while let Ok(detected) = app.drop.rx.try_recv() {
        app.drop.pending = Some(detected);
        app.drop.busy = false;
    }

    let (hovered, dropped): (bool, Vec<PathBuf>) = ctx.input(|i| {
        let hovered = !i.raw.hovered_files.is_empty();
        let dropped = i
            .raw
            .dropped_files
            .iter()
            .map(|f| f.path().to_path_buf())
            .collect();
        (hovered, dropped)
    });
    app.drop.hovering = hovered;

    if app.drop.pending.is_none()
        && !app.drop.busy
        && let Some(folder) = dropped.into_iter().find(|p| p.is_dir())
    {
        app.drop.busy = true;
        let tx = app.drop.tx.clone();
        let ctx2 = ctx.clone();
        std::thread::spawn(move || {
            let detected = importer::detect(&folder);
            let _ = tx.send(detected);
            ctx2.request_repaint();
        });
    }

    if app.drop.hovering && app.drop.pending.is_none() {
        let oid = ui.make_persistent_id("drop_overlay");
        overlay(&ctx, oid, frame, s);
    }
    confirm(app, ui, s);
}

fn overlay(ctx: &egui::Context, id: egui::Id, frame: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    egui::Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(frame.min)
        .show(ctx, |ui| {
            let painter = ui.painter();
            painter.rect_filled(
                frame,
                theme::corner(px(theme::RADIUS_APP)),
                theme::with_alpha(Color32::BLACK, 175),
            );
            let inset = frame.shrink(px(22.0));
            dashed_rect(painter, inset, theme::INK, px(12.0), px(8.0), px(1.5));
            painter.text(
                inset.center() - Vec2::new(0.0, px(14.0)),
                Align2::CENTER_CENTER,
                theme::icon::PACKAGE.to_string(),
                theme::icon_font(px(34.0), false),
                theme::INK,
            );
            painter.text(
                inset.center() + Vec2::new(0.0, px(28.0)),
                Align2::CENTER_CENTER,
                "Drop a game folder to create a project",
                FontId::new(px(15.0), theme::family_semibold()),
                theme::INK,
            );
        });
}

fn dashed_rect(
    painter: &egui::Painter,
    rect: Rect,
    color: Color32,
    dash: f32,
    gap: f32,
    width: f32,
) {
    let stroke = Stroke::new(width, color);
    let step = dash + gap;
    let edge = |a: Pos2, b: Pos2| {
        let len = a.distance(b);
        let dir = (b - a) / len.max(1e-3);
        let mut t = 0.0;
        while t < len {
            let start = a + dir * t;
            let end = a + dir * (t + dash).min(len);
            painter.line_segment([start, end], stroke);
            t += step;
        }
    };
    edge(rect.left_top(), rect.right_top());
    edge(rect.right_top(), rect.right_bottom());
    edge(rect.right_bottom(), rect.left_bottom());
    edge(rect.left_bottom(), rect.left_top());
}

fn confirm(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let Some(detected) = app.drop.pending.clone() else {
        return;
    };
    let id = ui.make_persistent_id("drop_confirm");
    let mut open = true;
    let mut create = false;
    let mut cancel = false;
    kit::modal(
        ui,
        id,
        &mut open,
        Vec2::new(px(560.0), px(420.0)),
        s,
        |ui| {
            let d = ui.max_rect();
            let x = d.left() + px(26.0);
            let painter = ui.painter().clone();
            kit::label(
                &painter,
                Pos2::new(x, d.top() + px(24.0)),
                "New project from folder",
                s,
            );
            painter.text(
                Pos2::new(d.right() - px(26.0), d.top() + px(28.0)),
                Align2::RIGHT_CENTER,
                detected.engine.label(),
                FontId::new(px(12.5), theme::family_semibold()),
                theme::INK_DIM,
            );

            let mut y = d.top() + px(58.0);
            for (k, v) in info_lines(&detected) {
                painter.text(
                    Pos2::new(x, y),
                    Align2::LEFT_TOP,
                    k,
                    FontId::new(px(11.5), theme::family_label()),
                    theme::INK_MUTED,
                );
                painter.text(
                    Pos2::new(x + px(120.0), y),
                    Align2::LEFT_TOP,
                    &v,
                    FontId::new(px(12.0), theme::family_medium()),
                    theme::INK_DIM,
                );
                y += px(22.0);
            }

            let dl = Rect::from_min_max(
                Pos2::new(x, y + px(8.0)),
                Pos2::new(d.right() - px(26.0), d.bottom() - px(80.0)),
            );
            kit::bordered(
                &painter,
                dl,
                theme::FIELD,
                theme::LINE,
                px(theme::RADIUS_FIELD),
            );
            painter.text(
                Pos2::new(dl.left() + px(12.0), dl.top() + px(10.0)),
                Align2::LEFT_TOP,
                format!("DEPOTS ({})", detected.depots.len()),
                FontId::new(px(11.0), theme::family_label()),
                theme::INK_MUTED,
            );
            for (i, dep) in detected.depots.iter().take(5).enumerate() {
                let os = dep
                    .os
                    .map(|o| format!("  ·  {}", o.label()))
                    .unwrap_or_default();
                painter.text(
                    Pos2::new(
                        dl.left() + px(12.0),
                        dl.top() + px(30.0) + i as f32 * px(18.0),
                    ),
                    Align2::LEFT_TOP,
                    format!("{}{os}", dep.name),
                    FontId::new(px(12.0), theme::family_medium()),
                    theme::INK_DIM,
                );
            }

            let by = d.bottom() - px(26.0) - px(40.0);
            let bw = px(160.0);
            if Button::new("Cancel", ButtonKind::Secondary)
                .show(
                    ui,
                    Rect::from_min_size(Pos2::new(x, by), Vec2::new(bw, px(40.0))),
                    s,
                )
                .clicked()
            {
                cancel = true;
            }
            if Button::new("Create project", ButtonKind::Primary)
                .icon(theme::icon::PLUS)
                .show(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(d.right() - px(26.0) - bw, by),
                        Vec2::new(bw, px(40.0)),
                    ),
                    s,
                )
                .clicked()
            {
                create = true;
            }
            create || cancel
        },
    );

    if create {
        let project = importer::to_project(&detected);
        match crate::config::save_project(&project) {
            Ok(()) => {
                app.projects.push(project.clone());
                app.projects_ui.select(&project);
                app.nav_request = Some(Page::Projects);
                app.toasts.push(
                    kit::ToastLevel::Success,
                    format!("Created {}", project.name),
                );
            }
            Err(e) => app.toasts.push(
                kit::ToastLevel::Error,
                format!("Could not create project: {e}"),
            ),
        }
        app.drop.pending = None;
    } else if cancel || !open {
        app.drop.pending = None;
    }
}

fn info_lines(d: &Detected) -> Vec<(&'static str, String)> {
    let full = d.content_root.to_string_lossy();
    let count = full.chars().count();
    let root = if count > 56 {
        let tail: String = full.chars().skip(count - 53).collect();
        format!("...{tail}")
    } else {
        full.to_string()
    };
    vec![
        ("Name", d.name.clone()),
        ("Engine", d.engine.label().to_string()),
        ("Content root", root),
        (
            "Executables",
            if d.executables.is_empty() {
                "none detected".to_string()
            } else {
                d.executables.join(", ")
            },
        ),
    ]
}
