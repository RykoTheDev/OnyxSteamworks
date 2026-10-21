use std::sync::mpsc::{Receiver, Sender};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, UiBuilder, Vec2};

use crate::model::Project;
use crate::ui::kit;
use crate::ui::kit::rail::RailAction;
use crate::ui::layout::{self, Layout};
use crate::ui::{anim, nav, pages, theme};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Page {
    #[default]
    Home,
    Projects,
    Build,
    History,
    Pipelines,
    Settings,
}

pub enum AppEvent {}

pub struct App {
    page: Page,
    prev_page: Option<Page>,
    transition: Option<anim::Transition>,
    pub projects: Vec<Project>,
    pub(crate) sub_tabs: [usize; 6],
    pub(crate) home_gen: u32,
    pub(crate) home_ui: pages::home::HomeState,
    pub(crate) projects_ui: pages::projects::ProjectsState,
    pub(crate) build_ui: pages::build::BuildState,
    pub(crate) history_ui: pages::history::HistoryState,
    pub(crate) backup_ui: pages::history::BackupState,
    pub(crate) patchnotes_ui: pages::history::PatchNotesState,
    pub(crate) analytics_ui: pages::history::AnalyticsState,
    pub(crate) pipeline_ui: pages::pipelines::PipelineState,
    pub(crate) queue: pages::pipelines::QueueState,
    pub(crate) doctor_ui: pages::settings::DoctorState,
    pub(crate) advanced_ui: pages::settings::AdvancedState,
    pub(crate) nav_request: Option<Page>,
    pub(crate) palette: crate::ui::palette::PaletteState,
    pub(crate) drop: crate::ui::dropimport::DropState,
    pub(crate) toasts: kit::Toasts,
    #[allow(dead_code)]
    event_tx: Sender<AppEvent>,
    event_rx: Receiver<AppEvent>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply_theme(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let projects = match crate::config::list_projects() {
            Ok(projects) => projects,
            Err(e) => {
                eprintln!("failed to load projects: {e}");
                Vec::new()
            }
        };

        let (event_tx, event_rx) = std::sync::mpsc::channel();
        Self {
            page: Page::default(),
            prev_page: None,
            transition: None,
            projects,
            sub_tabs: [0; 6],
            home_gen: 0,
            home_ui: pages::home::HomeState::new(),
            projects_ui: pages::projects::ProjectsState::new(),
            build_ui: pages::build::BuildState::new(),
            history_ui: pages::history::HistoryState::new(),
            backup_ui: pages::history::BackupState::new(),
            patchnotes_ui: pages::history::PatchNotesState::new(),
            analytics_ui: pages::history::AnalyticsState::new(),
            pipeline_ui: pages::pipelines::PipelineState::new(),
            queue: pages::pipelines::QueueState::new(),
            doctor_ui: pages::settings::DoctorState::new(),
            advanced_ui: pages::settings::AdvancedState::new(),
            nav_request: None,
            palette: crate::ui::palette::PaletteState::new(),
            drop: crate::ui::dropimport::DropState::new(),
            toasts: kit::Toasts::default(),
            event_tx,
            event_rx,
        }
    }

    fn goto(&mut self, ctx: &egui::Context, page: Page) {
        if page == self.page {
            return;
        }
        self.prev_page = Some(self.page);
        self.page = page;
        self.transition = Some(anim::Transition::new(ctx, 0.26, anim::ease_in_out_cubic));
        if page == Page::Home {
            self.home_gen = self.home_gen.wrapping_add(1);
        }
    }

    fn render_page(&mut self, ui: &mut egui::Ui, page: Page, lay: &Layout, clip: egui::Rect, opacity: f32) {
        let mut cu = ui.new_child(UiBuilder::new().max_rect(ui.max_rect()));
        cu.set_opacity(opacity);
        cu.set_clip_rect(clip);
        pages::render(self, &mut cu, page, lay);
    }

    fn search_pill(&mut self, ui: &mut egui::Ui, content: Rect, s: f32) {
        let px = |v: f32| theme::px(v, s);
        let pw = px(152.0);
        let ph = px(32.0);
        let r = Rect::from_min_size(Pos2::new(content.right() - pw, content.top()), Vec2::new(pw, ph));
        let id = ui.make_persistent_id("search_pill");
        let resp = ui.interact(r, id, Sense::click());
        let ctx = ui.ctx().clone();
        let hover = anim::anim_bool(&ctx, id.with("h"), resp.hovered(), 0.12, anim::ease_out_cubic);
        let painter = ui.painter();
        painter.rect_filled(r, theme::corner(px(theme::RADIUS_FIELD)), theme::lerp_color(theme::FIELD, theme::FIELD_HI, hover));
        painter.rect_stroke(r, theme::corner(px(theme::RADIUS_FIELD)), Stroke::new(1.0, theme::LINE), StrokeKind::Inside);
        painter.text(Pos2::new(r.left() + px(12.0), r.center().y), Align2::LEFT_CENTER, theme::icon::MAGNIFYING_GLASS.to_string(), theme::icon_font(px(14.0), false), theme::INK_MUTED);
        painter.text(Pos2::new(r.left() + px(32.0), r.center().y), Align2::LEFT_CENTER, "Search", FontId::new(px(12.5), theme::family_medium()), theme::INK_DIM);
        painter.text(Pos2::new(r.right() - px(12.0), r.center().y), Align2::RIGHT_CENTER, "Ctrl+K", FontId::new(px(11.0), theme::family_medium()), theme::INK_MUTED);
        if resp.clicked() {
            self.palette.toggle();
        }
    }

    #[allow(clippy::never_loop)]
    fn pump_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {}
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump_events();
        self.queue.tick(&self.projects, ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            let lay = layout::compute(ui.max_rect());
            let s = lay.scale;

            kit::bordered(ui.painter(), lay.frame, theme::SURFACE, theme::LINE, theme::px(theme::RADIUS_APP, s));
            ui.painter().line_segment(
                [
                    Pos2::new(lay.rail.right(), lay.frame.top() + theme::px(14.0, s)),
                    Pos2::new(lay.rail.right(), lay.frame.bottom() - theme::px(14.0, s)),
                ],
                Stroke::new(1.0, theme::LINE),
            );

            if let Some(RailAction::Navigate(page)) = kit::rail(ui, lay.rail, self.page, s) {
                self.goto(&ctx, page);
            }

            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
                self.palette.toggle();
            }

            let mut nav_to = None;
            if !self.palette.open {
                for item in nav::ITEMS.iter() {
                    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(item.shortcut)) {
                        nav_to = Some(item.page);
                    }
                }
            }
            if let Some(page) = nav_to {
                self.goto(&ctx, page);
            }

            let clip = Rect::from_min_max(
                Pos2::new(lay.rail.right(), lay.frame.top()),
                lay.frame.max,
            );
            let slide = theme::px(14.0, s);
            if let Some(tr) = self.transition {
                let t = tr.progress(&ctx);
                if let Some(prev) = self.prev_page {
                    let l = lay.translated(Vec2::new(-slide * t, 0.0));
                    self.render_page(ui, prev, &l, clip, 1.0 - t);
                }
                let l = lay.translated(Vec2::new(slide * (1.0 - t), 0.0));
                self.render_page(ui, self.page, &l, clip, t);
                if tr.is_done(&ctx) {
                    self.transition = None;
                    self.prev_page = None;
                }
            } else {
                self.render_page(ui, self.page, &lay, clip, 1.0);
            }

            self.search_pill(ui, lay.content, s);
            crate::ui::dropimport::show(self, ui, lay.frame, s);
            crate::ui::palette::show(self, ui, s);

            if let Some(page) = self.nav_request.take() {
                self.goto(&ctx, page);
            }

            let toast_anchor = lay.toast_anchor + Vec2::new(0.0, theme::px(42.0, s));
            self.toasts.show(ui, toast_anchor, s);
        });
    }
}

