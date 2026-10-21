use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;

use crate::model::Project;
use crate::ui::pages;

/// The page currently shown in the main content area.
///
/// Variants other than `Home` are selected once navigation exists (a later
/// step); they are read by the router below.
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

/// Messages sent to the UI thread from background work.
///
/// Background threads clone [`App::event_tx`], send an [`AppEvent`], then call
/// `ctx.request_repaint()` so the UI drains it on the next frame. No variants
/// exist yet; real events arrive in later steps.
pub enum AppEvent {}

/// Root application state and the UI-side end of the background message pump.
pub struct App {
    page: Page,
    /// Projects loaded from disk at startup.
    pub projects: Vec<Project>,
    /// Debug-only Animation Lab, enabled by the `ONYX_ANIM_LAB` env var.
    #[cfg(debug_assertions)]
    show_anim_lab: bool,
    /// Handed to background threads so they can report progress. Unused until a
    /// background worker exists.
    #[allow(dead_code)]
    event_tx: Sender<AppEvent>,
    event_rx: Receiver<AppEvent>,
}

impl App {
    /// Build the app, apply the theme, and load projects.
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
            #[cfg(debug_assertions)]
            show_anim_lab: std::env::var_os("ONYX_ANIM_LAB").is_some(),
            event_tx,
            event_rx,
        }
    }

    // `never_loop` fires only because `AppEvent` is still uninhabited; the loop
    // becomes a real drain the moment the first variant is added.
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
        egui::CentralPanel::default().show(ui, |ui| {
            #[cfg(debug_assertions)]
            if self.show_anim_lab {
                pages::anim_lab::show(self, ui);
                return;
            }
            match self.page {
                Page::Home => pages::home::show(self, ui),
                Page::Projects => pages::projects::show(self, ui),
                Page::History => pages::history::show(self, ui),
                Page::Pipelines => pages::pipelines::show(self, ui),
                Page::Settings => pages::settings::show(self, ui),
            }
        });
    }
}
