mod chrome;
mod logic;
mod steps;

use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Pos2, Rect, Vec2};

use crate::app::App;
use crate::features::importer::Detected;
use crate::ui::kit;
use crate::ui::theme;

pub const STEPS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PickTarget {
    Install,
    Game,
}

#[derive(Default)]
pub(super) struct Intent {
    next: bool,
    back: bool,
    skip: bool,
    close: bool,
    pick_install: bool,
    pick_game: bool,
    start_install: bool,
    create_project: bool,
    authorize: bool,
    preview_build: bool,
}

pub(super) struct View {
    pub step: usize,
    pub install_dir: String,
    pub downloading: bool,
    pub dl_done: u64,
    pub dl_total: u64,
    pub extracting: bool,
    pub installed: Option<String>,
    pub dl_error: Option<String>,
    pub game_dir: String,
    pub detecting: bool,
    pub detected: Option<(String, String, usize)>,
    pub created: Option<String>,
    pub authorized: bool,
    pub account: String,
}

pub struct WizardState {
    pub open: bool,
    step: usize,
    prev_open: bool,
    install_dir: String,
    dl_rx: Option<Receiver<crate::features::wizard::DownloadMsg>>,
    dl_done: u64,
    dl_total: u64,
    extracting: bool,
    installed: Option<String>,
    dl_error: Option<String>,
    game_dir: String,
    detecting: bool,
    detected: Option<Detected>,
    created: Option<String>,
    authorized: bool,
    pick_tx: Sender<(PickTarget, String)>,
    pick_rx: Receiver<(PickTarget, String)>,
    detect_tx: Sender<Detected>,
    detect_rx: Receiver<Detected>,
}

impl WizardState {
    pub fn new(open: bool) -> Self {
        let (pick_tx, pick_rx) = channel();
        let (detect_tx, detect_rx) = channel();
        Self {
            open,
            step: 0,
            prev_open: open,
            install_dir: String::new(),
            dl_rx: None,
            dl_done: 0,
            dl_total: 0,
            extracting: false,
            installed: None,
            dl_error: None,
            game_dir: String::new(),
            detecting: false,
            detected: None,
            created: None,
            authorized: false,
            pick_tx,
            pick_rx,
            detect_tx,
            detect_rx,
        }
    }

    pub fn start(&mut self) {
        self.open = true;
        self.step = 0;
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, s: f32) {
    let ctx = ui.ctx().clone();
    logic::pump(app, &ctx);

    let view = snapshot(app);
    let mut intent = Intent::default();
    let id = ui.make_persistent_id("setup_wizard");
    let px = |v: f32| theme::px(v, s);
    let size = Vec2::new(px(624.0), px(508.0));
    let busy = view.downloading || view.extracting || view.detecting;

    kit::modal_with(ui, id, &mut app.wizard.open, size, s, !busy, |ui| {
        let d = ui.max_rect();
        chrome::chrome(ui, d, &view, &mut intent, s);
        let body = Rect::from_min_max(
            Pos2::new(d.left() + px(28.0), d.top() + px(116.0)),
            Pos2::new(d.right() - px(28.0), d.bottom() - px(78.0)),
        );
        steps::body(ui, body, &view, &mut intent, s);
        chrome::footer(ui, d, &view, &mut intent, s);
        false
    });

    logic::apply(app, ui, intent, &ctx);

    if app.wizard.prev_open && !app.wizard.open {
        logic::mark_done();
    }
    app.wizard.prev_open = app.wizard.open;
}

fn snapshot(app: &App) -> View {
    let w = &app.wizard;
    let detected = w
        .detected
        .as_ref()
        .map(|d| (d.name.clone(), d.engine.label().to_string(), d.depots.len()));
    View {
        step: w.step,
        install_dir: w.install_dir.clone(),
        downloading: w.dl_rx.is_some() && !w.extracting,
        dl_done: w.dl_done,
        dl_total: w.dl_total,
        extracting: w.extracting,
        installed: w.installed.clone(),
        dl_error: w.dl_error.clone(),
        game_dir: w.game_dir.clone(),
        detecting: w.detecting,
        detected,
        created: w.created.clone(),
        authorized: w.authorized,
        account: logic::wizard_account(app),
    }
}
