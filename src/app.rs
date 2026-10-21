use std::sync::mpsc::{Receiver, Sender};

use eframe::egui::{self, UiBuilder, Vec2};

use crate::model::Project;
use crate::ui::bento::rail;
use crate::ui::layout::{self, Layout};
use crate::ui::{anim, nav, pages, theme};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Page {
    #[default]
    Home,
    Projects,
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
    pub(crate) build_desc: String,
    pub(crate) search: String,
    pub(crate) branch_sel: Vec<usize>,
    pub(crate) sub_tabs: [usize; 5],
    pub(crate) home_gen: u32,
    #[cfg(debug_assertions)]
    show_anim_lab: bool,
    #[cfg(debug_assertions)]
    pub(crate) lab: pages::anim_lab::LabState,
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
            build_desc: String::new(),
            search: String::new(),
            branch_sel: vec![0],
            sub_tabs: [0; 5],
            home_gen: 0,
            #[cfg(debug_assertions)]
            show_anim_lab: std::env::var_os("ONYX_ANIM_LAB").is_some(),
            #[cfg(debug_assertions)]
            lab: pages::anim_lab::LabState::default(),
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
        self.transition = Some(anim::Transition::new(ctx, 0.28, anim::ease_in_out_cubic));
        if page == Page::Home {
            self.home_gen = self.home_gen.wrapping_add(1);
        }
    }

    fn render_page(&mut self, ui: &mut egui::Ui, page: Page, lay: &Layout, opacity: f32) {
        let mut cu = ui.new_child(UiBuilder::new().max_rect(ui.max_rect()));
        cu.set_opacity(opacity);
        pages::render(self, &mut cu, page, lay);
    }

    #[allow(clippy::never_loop)]
    fn pump_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {}
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump_events();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            #[cfg(debug_assertions)]
            if self.show_anim_lab {
                pages::anim_lab::show(self, ui);
                return;
            }

            let lay = layout::compute(ui.max_rect());

            if let Some(rail::RailAction::Navigate(page)) =
                rail::rail(ui, lay.rail, self.page, lay.scale)
            {
                self.goto(&ctx, page);
            }

            let mut nav_to = None;
            for item in nav::ITEMS.iter() {
                if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(item.shortcut)) {
                    nav_to = Some(item.page);
                }
            }
            if let Some(page) = nav_to {
                self.goto(&ctx, page);
            }

            let slide = theme::px(12.0, lay.scale);
            if let Some(tr) = self.transition {
                let t = tr.progress(&ctx);
                if let Some(prev) = self.prev_page {
                    let l = lay.translated(Vec2::new(-slide * t, 0.0));
                    self.render_page(ui, prev, &l, 1.0 - t);
                }
                let l = lay.translated(Vec2::new(slide * (1.0 - t), 0.0));
                self.render_page(ui, self.page, &l, t);
                if tr.is_done(&ctx) {
                    self.transition = None;
                    self.prev_page = None;
                }
            } else {
                self.render_page(ui, self.page, &lay, 1.0);
            }
        });
    }
}
