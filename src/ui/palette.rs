use eframe::egui::{self, Align2, FontId, Frame, Key, Pos2, Rect, TextEdit, UiBuilder, Vec2};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::app::{App, Page};
use crate::features::actions::{Action, Command, registry};
use crate::features::quick;
use crate::ui::kit;
use crate::ui::{nav, theme};

pub struct PaletteState {
    pub open: bool,
    query: String,
    selected: usize,
    recent: Vec<String>,
    focus: bool,
}

impl PaletteState {
    pub fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            selected: 0,
            recent: Vec::new(),
            focus: false,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.query.clear();
            self.selected = 0;
            self.focus = true;
        }
    }
}

fn rank(actions: &[Action], query: &str, recent: &[String]) -> Vec<usize> {
    if query.trim().is_empty() {
        let mut order = Vec::new();
        for rid in recent {
            if let Some(i) = actions.iter().position(|a| &a.id == rid) {
                order.push(i);
            }
        }
        for i in 0..actions.len() {
            if !order.contains(&i) {
                order.push(i);
            }
        }
        order
    } else {
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(usize, i64)> = actions
            .iter()
            .enumerate()
            .filter_map(|(i, a)| matcher.fuzzy_match(&a.haystack(), query).map(|sc| (i, sc)))
            .collect();
        scored.sort_by_key(|&(_, sc)| std::cmp::Reverse(sc));
        scored.into_iter().map(|(i, _)| i).collect()
    }
}

fn push_recent(recent: &mut Vec<String>, id: &str) {
    recent.retain(|r| r != id);
    recent.insert(0, id.to_string());
    recent.truncate(8);
}

pub fn show(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let actions = registry(&app.projects);
    let ranked = rank(&actions, &app.palette.query, &app.palette.recent);

    let mut pick: Option<usize> = None;
    if app.palette.open {
        ctx.input(|i| {
            if i.key_pressed(Key::ArrowDown) {
                app.palette.selected += 1;
            }
            if i.key_pressed(Key::ArrowUp) {
                app.palette.selected = app.palette.selected.saturating_sub(1);
            }
        });
        if ranked.is_empty() {
            app.palette.selected = 0;
        } else {
            app.palette.selected = app.palette.selected.min(ranked.len() - 1);
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            pick = ranked.get(app.palette.selected).copied();
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            app.palette.open = false;
        }
    }

    let id = ui.make_persistent_id("cmd_palette");
    let mut open = app.palette.open;
    let size = Vec2::new(px(560.0), px(428.0));
    let pal = &mut app.palette;
    let mut clicked: Option<usize> = None;
    kit::modal(ui, id, &mut open, size, s, |ui| {
        let d = ui.max_rect();
        let x = d.left() + px(20.0);
        let w = d.width() - px(40.0);

        let field = Rect::from_min_size(Pos2::new(x, d.top() + px(18.0)), Vec2::new(w, px(40.0)));
        kit::surface(
            &ui.painter().clone(),
            field,
            theme::FIELD,
            px(theme::RADIUS_FIELD),
        );
        ui.painter().text(
            Pos2::new(field.left() + px(14.0), field.center().y),
            Align2::LEFT_CENTER,
            theme::icon::MAGNIFYING_GLASS.to_string(),
            theme::icon_font(px(15.0), false),
            theme::INK_MUTED,
        );
        let inner = Rect::from_min_max(
            Pos2::new(field.left() + px(38.0), field.top()),
            Pos2::new(field.right() - px(12.0), field.bottom()),
        );
        let resp = ui.put(
            inner,
            TextEdit::singleline(&mut pal.query)
                .frame(Frame::NONE)
                .margin(egui::Margin::ZERO)
                .align(Align2::LEFT_CENTER)
                .hint_text("Type a command or page...")
                .text_color(theme::INK)
                .desired_width(f32::INFINITY),
        );
        if resp.changed() {
            pal.selected = 0;
        }
        if pal.focus {
            resp.request_focus();
            pal.focus = false;
        }

        let list = Rect::from_min_max(
            Pos2::new(x, field.bottom() + px(14.0)),
            Pos2::new(d.right() - px(20.0), d.bottom() - px(16.0)),
        );
        let mut cu = ui.new_child(UiBuilder::new().max_rect(list));
        cu.set_clip_rect(list);
        egui::ScrollArea::vertical()
            .id_salt("palette_list")
            .auto_shrink([false, false])
            .show(&mut cu, |ui| {
                ui.set_width(list.width());
                if ranked.is_empty() {
                    ui.painter().text(
                        Pos2::new(list.left() + px(6.0), list.top() + px(8.0)),
                        Align2::LEFT_TOP,
                        "No matching commands.",
                        FontId::new(px(12.5), theme::family_medium()),
                        theme::INK_MUTED,
                    );
                }
                for (row, &ai) in ranked.iter().enumerate() {
                    let a = &actions[ai];
                    let (r, sense) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), px(38.0)),
                        egui::Sense::click(),
                    );
                    let on = row == pal.selected;
                    let hover = sense.hovered();
                    let fill = if on {
                        theme::FIELD_HI
                    } else if hover {
                        theme::FIELD
                    } else {
                        theme::SURFACE_2
                    };
                    kit::bordered(ui.painter(), r, fill, theme::LINE, px(theme::RADIUS_CARD));
                    ui.painter().text(
                        Pos2::new(r.left() + px(14.0), r.center().y),
                        Align2::LEFT_CENTER,
                        &a.title,
                        FontId::new(px(13.0), theme::family_medium()),
                        theme::INK,
                    );
                    if let Some(sc) = &a.shortcut {
                        ui.painter().text(
                            Pos2::new(r.right() - px(14.0), r.center().y),
                            Align2::RIGHT_CENTER,
                            sc,
                            FontId::new(px(11.5), theme::family_medium()),
                            theme::INK_MUTED,
                        );
                    }
                    if sense.clicked() {
                        clicked = Some(ai);
                    }
                    ui.add_space(px(6.0));
                }
            });
        false
    });
    app.palette.open = open;

    if let Some(ai) = pick.or(clicked) {
        app.palette.open = false;
        let id = actions[ai].id.clone();
        let cmd = actions[ai].command.clone();
        push_recent(&mut app.palette.recent, &id);
        execute(app, &cmd, &ctx);
    }
    if app.palette.open {
        ctx.request_repaint();
    }
}

fn execute(app: &mut App, cmd: &Command, ctx: &egui::Context) {
    match cmd {
        Command::Goto(page, sub) => {
            app.sub_tabs[nav::index(*page)] = *sub;
            app.nav_request = Some(*page);
        }
        Command::SwitchProject(i) => {
            if let Some(p) = app.projects.get(*i) {
                let name = if p.name.is_empty() {
                    "project".to_string()
                } else {
                    p.name.clone()
                };
                app.build_ui.project_sel = *i;
                app.toasts
                    .push(kit::ToastLevel::Success, format!("Switched to {name}"));
            }
        }
        Command::StartBuild => start_build(app, false, ctx),
        Command::PreviewBuild => start_build(app, true, ctx),
        Command::CancelBuild => {
            if app.build_ui.running {
                app.build_ui.cancel();
            } else {
                app.toasts
                    .push(kit::ToastLevel::Warning, "No build is running");
            }
        }
        Command::OpenSteamworks => {
            if let Some(p) = app.projects.get(app.build_ui.project_sel) {
                let url = quick::steamworks_builds_url(&p.app_id);
                std::thread::spawn(move || {
                    let _ = open::that(url);
                });
            }
        }
        Command::RunDoctor => {
            app.doctor_ui.request();
            app.sub_tabs[nav::index(Page::Settings)] = 3;
            app.nav_request = Some(Page::Settings);
        }
        Command::ExportCi => {
            if let Some(p) = app.projects.get(app.build_ui.project_sel) {
                let yaml = crate::cli::ci_workflow_yaml(p);
                app.toasts
                    .push(kit::ToastLevel::Success, "Exporting CI workflow");
                std::thread::spawn(move || {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("steam-deploy.yml")
                        .save_file()
                    {
                        let _ = std::fs::write(path, yaml);
                    }
                });
            }
        }
    }
}

pub fn start_build(app: &mut App, preview: bool, ctx: &egui::Context) {
    let Some(project) = app.projects.get(app.build_ui.project_sel).cloned() else {
        app.toasts
            .push(kit::ToastLevel::Warning, "No project to build");
        return;
    };
    if app.build_ui.running {
        app.toasts
            .push(kit::ToastLevel::Warning, "A build is already running");
        return;
    }
    let branch = project
        .branches
        .get(app.build_ui.branch_sel)
        .map(|b| b.name.clone())
        .unwrap_or_default();
    app.build_ui.preview_only = preview;
    match app.build_ui.start(&project, &branch, ctx) {
        Ok(()) => {
            app.nav_request = Some(Page::Build);
            app.toasts.push(
                kit::ToastLevel::Success,
                if preview {
                    "Starting preview build"
                } else {
                    "Starting build"
                },
            );
        }
        Err(e) => {
            app.build_ui
                .push_log(format!("Build could not start: {e}"), true);
            app.toasts
                .push(kit::ToastLevel::Error, "Build failed to start");
        }
    }
}
