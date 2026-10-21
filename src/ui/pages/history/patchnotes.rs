use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Align2, FontId, Frame, Pos2, Rect, TextEdit, UiBuilder, Vec2};

use crate::app::App;
use crate::features::patchnotes;
use crate::ui::kit::{self, Button, ButtonKind};
use crate::ui::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Markdown,
    Plain,
    Bbcode,
}

pub struct PatchNotesState {
    pub project_sel: usize,
    draft: String,
    format: Format,
    generating: bool,
    cancel: Arc<AtomicBool>,
    tx: Sender<Result<String, String>>,
    rx: Receiver<Result<String, String>>,
}

impl PatchNotesState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            project_sel: 0,
            draft: String::new(),
            format: Format::Markdown,
            generating: false,
            cancel: Arc::new(AtomicBool::new(false)),
            tx,
            rx,
        }
    }
}

fn preview_text(state: &PatchNotesState) -> String {
    match state.format {
        Format::Markdown => state.draft.clone(),
        Format::Plain => patchnotes::to_plain(&state.draft),
        Format::Bbcode => patchnotes::to_bbcode(&state.draft),
    }
}

pub fn start_gen(app: &mut App, llm: bool, ctx: &egui::Context) {
    app.history_ui.ensure_loaded();
    let Some(project) = app.projects.get(app.patchnotes_ui.project_sel).cloned() else {
        return;
    };
    let since = app.history_ui.last_commit_for(&project.id);
    app.patchnotes_ui.cancel = Arc::new(AtomicBool::new(false));
    app.patchnotes_ui.generating = true;
    let cancel = app.patchnotes_ui.cancel.clone();
    let tx = app.patchnotes_ui.tx.clone();
    let cfg = crate::config::load_settings().unwrap_or_default();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let commits = patchnotes::commits_since(&project, since.as_deref());
        let result = if commits.is_empty() || !llm {
            Ok(patchnotes::group_markdown(&commits))
        } else {
            patchnotes::generate_llm(
                &cfg.llm_base_url,
                &cfg.llm_model,
                &cfg.llm_api_key,
                &commits,
                &cancel,
            )
        };
        let _ = tx.send(result);
        ctx.request_repaint();
    });
}

fn drain(app: &mut App, ctx: &egui::Context) {
    while let Ok(res) = app.patchnotes_ui.rx.try_recv() {
        app.patchnotes_ui.generating = false;
        match res {
            Ok(text) => {
                app.patchnotes_ui.draft = text;
                app.toasts.push(kit::ToastLevel::Success, "Draft ready");
            }
            Err(e) if e == "cancelled" => app
                .toasts
                .push(kit::ToastLevel::Warning, "Generation cancelled"),
            Err(e) => app
                .toasts
                .push(kit::ToastLevel::Error, format!("Generate failed: {e}")),
        }
    }
    if app.patchnotes_ui.generating {
        ctx.request_repaint();
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    drain(app, &ctx);
    app.history_ui.ensure_loaded();

    if app.projects.is_empty() {
        kit::empty_state(
            ui,
            rect,
            theme::icon::SPARKLE,
            "No project yet",
            "Create a project to draft patch notes from git.",
            s,
        );
        return;
    }
    if app.patchnotes_ui.project_sel >= app.projects.len() {
        app.patchnotes_ui.project_sel = 0;
    }

    let cfg = crate::config::load_settings().unwrap_or_default();
    project_chips(app, ui, rect, rect.top(), s);
    controls(app, ui, rect, rect.top() + px(40.0), &cfg, &ctx, s);
    if cfg.llm_enabled {
        ui.painter().text(
            Pos2::new(rect.left(), rect.top() + px(84.0)),
            Align2::LEFT_CENTER,
            "AI mode sends commit messages to your configured endpoint.",
            FontId::new(px(11.5), theme::family_medium()),
            theme::INK_MUTED,
        );
    }

    let editor_top = rect.top() + px(102.0);
    let editor_bottom = rect.bottom() - px(52.0);
    let gap = px(16.0);
    let col_w = (rect.width() - gap) / 2.0;
    let edit_rect = Rect::from_min_max(
        Pos2::new(rect.left(), editor_top),
        Pos2::new(rect.left() + col_w, editor_bottom),
    );
    let prev_rect = Rect::from_min_max(
        Pos2::new(edit_rect.right() + gap, editor_top),
        Pos2::new(rect.right(), editor_bottom),
    );
    editor(app, ui, edit_rect, s);
    preview(app, ui, prev_rect, s);
    actions(app, ui, rect, editor_bottom + px(12.0), &ctx, s);
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
            i == app.patchnotes_ui.project_sel,
            s,
        )
        .clicked()
        {
            app.patchnotes_ui.project_sel = i;
        }
        x += size.x + px(8.0);
    }
}

fn controls(
    app: &mut App,
    ui: &mut egui::Ui,
    rect: Rect,
    y: f32,
    cfg: &crate::config::Settings,
    ctx: &egui::Context,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let formats = [
        ("Markdown", Format::Markdown),
        ("Plain", Format::Plain),
        ("BBCode", Format::Bbcode),
    ];
    let mut x = rect.left();
    for (label, val) in formats {
        let size = kit::chip_size(ui, label, s);
        if kit::chip(
            ui,
            Rect::from_min_size(Pos2::new(x, y + px(4.0)), size),
            label,
            app.patchnotes_ui.format == val,
            s,
        )
        .clicked()
        {
            app.patchnotes_ui.format = val;
        }
        x += size.x + px(8.0);
    }

    let bh = px(34.0);
    if app.patchnotes_ui.generating {
        let cancel = Rect::from_min_size(
            Pos2::new(rect.right() - px(150.0), y),
            Vec2::new(px(150.0), bh),
        );
        if Button::new("CANCEL", ButtonKind::Secondary)
            .icon(theme::icon::X)
            .show(ui, cancel, s)
            .clicked()
        {
            app.patchnotes_ui.cancel.store(true, Ordering::SeqCst);
        }
        return;
    }
    let git = Rect::from_min_size(
        Pos2::new(rect.right() - px(150.0), y),
        Vec2::new(px(150.0), bh),
    );
    if Button::new("GENERATE", ButtonKind::Primary)
        .icon(theme::icon::SPARKLE)
        .show(ui, git, s)
        .clicked()
    {
        start_gen(app, false, ctx);
    }
    if cfg.llm_enabled {
        let ai = Rect::from_min_size(
            Pos2::new(git.left() - px(8.0) - px(130.0), y),
            Vec2::new(px(130.0), bh),
        );
        if Button::new("AI DRAFT", ButtonKind::Secondary)
            .icon(theme::icon::SPARKLE)
            .show(ui, ai, s)
            .clicked()
        {
            start_gen(app, true, ctx);
        }
    }
}

fn editor(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::FIELD,
        theme::LINE,
        px(theme::RADIUS_FIELD),
    );
    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left() + px(12.0), rect.top() + px(10.0)),
        "EDITOR",
        s,
    );
    let inner = Rect::from_min_max(
        Pos2::new(rect.left() + px(12.0), rect.top() + px(34.0)),
        Pos2::new(rect.right() - px(10.0), rect.bottom() - px(10.0)),
    );
    let mut cu = ui.new_child(UiBuilder::new().max_rect(inner));
    cu.set_clip_rect(inner);
    egui::ScrollArea::vertical()
        .id_salt("patchnotes_editor")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            ui.set_width(inner.width());
            ui.add(
                TextEdit::multiline(&mut app.patchnotes_ui.draft)
                    .frame(Frame::NONE)
                    .hint_text("Generate a draft or write notes here...")
                    .font(FontId::new(px(12.5), theme::family_medium()))
                    .text_color(theme::INK)
                    .desired_width(f32::INFINITY)
                    .desired_rows(14),
            );
        });
}

fn preview(app: &mut App, ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    kit::bordered(
        &ui.painter().clone(),
        rect,
        theme::BG,
        theme::LINE,
        px(theme::RADIUS_FIELD),
    );
    kit::label(
        &ui.painter().clone(),
        Pos2::new(rect.left() + px(12.0), rect.top() + px(10.0)),
        "PREVIEW",
        s,
    );
    let text = preview_text(&app.patchnotes_ui);
    let inner = Rect::from_min_max(
        Pos2::new(rect.left() + px(12.0), rect.top() + px(34.0)),
        Pos2::new(rect.right() - px(10.0), rect.bottom() - px(10.0)),
    );
    let mut cu = ui.new_child(UiBuilder::new().max_rect(inner));
    cu.set_clip_rect(inner);
    egui::ScrollArea::vertical()
        .id_salt("patchnotes_preview")
        .auto_shrink([false, false])
        .show(&mut cu, |ui| {
            ui.set_width(inner.width());
            if text.trim().is_empty() {
                ui.label(
                    egui::RichText::new("Nothing to preview yet.")
                        .font(FontId::new(px(12.0), theme::family_medium()))
                        .color(theme::INK_MUTED),
                );
            } else {
                for line in text.lines() {
                    ui.label(
                        egui::RichText::new(line)
                            .monospace()
                            .size(px(11.5))
                            .color(theme::INK_DIM),
                    );
                }
            }
        });
}

fn actions(app: &mut App, ui: &mut egui::Ui, rect: Rect, y: f32, ctx: &egui::Context, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let bh = px(38.0);
    let has_text = !app.patchnotes_ui.draft.trim().is_empty();
    let copy = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(px(120.0), bh));
    if Button::new("COPY", ButtonKind::Secondary)
        .icon(theme::icon::COPY)
        .enabled(has_text)
        .show(ui, copy, s)
        .clicked()
        && has_text
    {
        ctx.copy_text(preview_text(&app.patchnotes_ui));
        app.toasts
            .push(kit::ToastLevel::Success, "Copied to clipboard");
    }
    let use_desc = Rect::from_min_size(
        Pos2::new(copy.right() + px(8.0), y),
        Vec2::new(px(188.0), bh),
    );
    if Button::new("USE AS DESCRIPTION", ButtonKind::Secondary)
        .enabled(has_text)
        .show(ui, use_desc, s)
        .clicked()
        && has_text
    {
        let first = app
            .patchnotes_ui
            .draft
            .lines()
            .map(|l| l.trim_start_matches("## ").trim_start_matches("- ").trim())
            .find(|l| !l.is_empty())
            .unwrap_or("")
            .to_string();
        app.build_ui.description = first;
        app.toasts
            .push(kit::ToastLevel::Success, "Set as build description");
    }

    let discord = Rect::from_min_size(
        Pos2::new(rect.right() - px(170.0), y),
        Vec2::new(px(170.0), bh),
    );
    let discord_on = crate::config::load_settings()
        .map(|c| c.discord_enabled)
        .unwrap_or(false);
    let can_discord = has_text && discord_on;
    if Button::new("SEND TO DISCORD", ButtonKind::Secondary)
        .enabled(can_discord)
        .disabled_reason("Enable Discord in Settings > Notifications")
        .show(ui, discord, s)
        .clicked()
        && can_discord
    {
        let webhook = crate::secrets::get_password(crate::features::notify::WEBHOOK_KEY)
            .ok()
            .flatten()
            .unwrap_or_default();
        app.notify_ui
            .post_text(webhook, app.patchnotes_ui.draft.clone(), ctx);
        app.toasts
            .push(kit::ToastLevel::Success, "Sending to Discord\u{2026}");
    }
    let save = Rect::from_min_size(
        Pos2::new(discord.left() - px(8.0) - px(130.0), y),
        Vec2::new(px(130.0), bh),
    );
    if Button::new("SAVE", ButtonKind::Primary)
        .icon(theme::icon::CHECK)
        .enabled(has_text)
        .show(ui, save, s)
        .clicked()
        && has_text
    {
        save_notes(app);
    }
}

fn save_notes(app: &mut App) {
    let Some(project) = app.projects.get(app.patchnotes_ui.project_sel).cloned() else {
        return;
    };
    let draft = app.patchnotes_ui.draft.clone();
    if let Some(mut rec) = app.history_ui.latest_for(&project.id).cloned() {
        rec.notes = Some(draft);
        app.history_ui.save(rec);
        app.toasts.push(
            kit::ToastLevel::Success,
            "Patch notes saved to latest build",
        );
    } else {
        app.toasts
            .push(kit::ToastLevel::Warning, "No build yet to attach notes to");
    }
}
