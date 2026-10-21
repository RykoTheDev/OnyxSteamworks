use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;

use crate::model::Project;
use crate::ui::pages;

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
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
    pub projects: Vec<Project>,
    #[allow(dead_code)]
    event_tx: Sender<AppEvent>,
    event_rx: Receiver<AppEvent>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::ui::theme::apply_theme(&cc.egui_ctx);

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
            projects,
            event_tx,
            event_rx,
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
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump_events();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| match self.page {
            Page::Home => pages::home::show(self, ui),
            Page::Projects => pages::projects::show(self, ui),
            Page::History => pages::history::show(self, ui),
            Page::Pipelines => pages::pipelines::show(self, ui),
            Page::Settings => pages::settings::show(self, ui),
        });
    }
}
