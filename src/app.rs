use std::sync::mpsc::{Receiver, Sender};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke, UiBuilder};

use crate::model::Project;
use crate::ui::kit;
use crate::ui::kit::rail::RailAction;
use crate::ui::layout::{self, Layout};
use crate::ui::{anim, nav, pages, theme};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Page {
    #[default]
    Projects,
    Build,
    History,
    Automation,
    Settings,
}

pub enum AppEvent {}

pub struct App {
    page: Page,
    prev_page: Option<Page>,
    transition: Option<anim::Transition>,
    pub projects: Vec<Project>,
    pub(crate) sub_tabs: [usize; 5],
    pub(crate) projects_ui: pages::projects::ProjectsState,
    pub(crate) build_ui: pages::build::BuildState,
    pub(crate) history_ui: pages::history::HistoryState,
    pub(crate) backup_ui: pages::history::BackupState,
    pub(crate) patchnotes_ui: pages::history::PatchNotesState,
    pub(crate) analytics_ui: pages::history::AnalyticsState,
    pub(crate) pipeline_ui: pages::automation::PipelineState,
    pub(crate) runs_ui: pages::automation::RunsState,
    pub(crate) scheduler: pages::automation::SchedulerState,
    pub(crate) notify_ui: crate::features::notify::NotifyState,
    pub(crate) doctor_ui: pages::settings::DoctorState,
    pub(crate) advanced_ui: pages::settings::AdvancedState,
    pub(crate) developer_ui: pages::settings::DeveloperState,
    pub(crate) general_ui: pages::settings::GeneralState,
    pub(crate) accounts_ui: pages::settings::AccountsState,
    pub(crate) notifications_ui: pages::settings::NotificationsState,
    pub(crate) account_menu: bool,
    pub(crate) nav_request: Option<Page>,
    pub(crate) palette: crate::ui::palette::PaletteState,
    pub(crate) drop: crate::ui::dropimport::DropState,
    pub(crate) wizard: crate::ui::wizard::WizardState,
    pub(crate) confirm_quit: bool,
    pub(crate) quitting: bool,
    pub(crate) toasts: kit::Toasts,
    #[allow(dead_code)]
    event_tx: Sender<AppEvent>,
    event_rx: Receiver<AppEvent>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply_theme(&cc.egui_ctx);
        crate::ui::window::round_corners(cc);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let startup = crate::config::load_settings().unwrap_or_default();
        anim::set_reduce_motion(startup.reduce_motion);
        if startup.ui_scale > 0.0 {
            theme::set_ui_scale(startup.ui_scale);
        }
        let _ = crate::features::maintenance::clean_old_logs(startup.log_retention_days);
        let open_wizard = !startup.wizard_done;

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
            sub_tabs: [0; 5],
            projects_ui: pages::projects::ProjectsState::new(),
            build_ui: pages::build::BuildState::new(),
            history_ui: pages::history::HistoryState::new(),
            backup_ui: pages::history::BackupState::new(),
            patchnotes_ui: pages::history::PatchNotesState::new(),
            analytics_ui: pages::history::AnalyticsState::new(),
            pipeline_ui: pages::automation::PipelineState::new(),
            runs_ui: pages::automation::RunsState::default(),
            scheduler: pages::automation::SchedulerState::new(),
            notify_ui: crate::features::notify::NotifyState::new(),
            doctor_ui: pages::settings::DoctorState::new(),
            advanced_ui: pages::settings::AdvancedState::new(),
            developer_ui: pages::settings::DeveloperState::default(),
            general_ui: pages::settings::GeneralState::default(),
            accounts_ui: pages::settings::AccountsState::default(),
            notifications_ui: pages::settings::NotificationsState::default(),
            account_menu: false,
            nav_request: None,
            palette: crate::ui::palette::PaletteState::new(),
            drop: crate::ui::dropimport::DropState::new(),
            wizard: crate::ui::wizard::WizardState::new(open_wizard),
            confirm_quit: false,
            quitting: false,
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
    }

    fn render_page(
        &mut self,
        ui: &mut egui::Ui,
        page: Page,
        lay: &Layout,
        clip: egui::Rect,
        opacity: f32,
    ) {
        let mut cu = ui.new_child(
            UiBuilder::new()
                .max_rect(ui.max_rect())
                .id_salt(nav::item(page).label),
        );
        cu.set_opacity(opacity);
        cu.set_clip_rect(clip);
        pages::render(self, &mut cu, page, lay);
    }

    fn header_bar(&self, ui: &mut egui::Ui, lay: &Layout) {
        let s = lay.scale;
        let px = |v: f32| theme::px(v, s);
        let h = lay.header;
        let painter = ui.painter();
        painter.line_segment(
            [
                Pos2::new(h.left(), h.bottom()),
                Pos2::new(h.right(), h.bottom()),
            ],
            Stroke::new(1.0, theme::LINE_SOFT),
        );
        let item = nav::item(self.page);
        let mut x = h.left() + px(20.0);
        painter.text(
            Pos2::new(x + px(7.0), h.center().y),
            Align2::CENTER_CENTER,
            item.glyph.to_string(),
            theme::icon_font(px(15.0), false),
            theme::INK_MUTED,
        );
        x += px(24.0);
        let crumb = FontId::new(px(13.0), theme::family_medium());
        let sub = item
            .sub_tabs
            .get(self.sub_tabs[nav::index(self.page)])
            .copied();
        let page_color = if sub.is_some() {
            theme::INK_MUTED
        } else {
            theme::INK
        };
        let r = painter.text(
            Pos2::new(x, h.center().y),
            Align2::LEFT_CENTER,
            item.label,
            crumb.clone(),
            page_color,
        );
        if let Some(sub) = sub {
            let chevron = painter.text(
                Pos2::new(r.right() + px(8.0), h.center().y),
                Align2::LEFT_CENTER,
                kit::rail::CARET_RIGHT.to_string(),
                theme::icon_font(px(11.0), false),
                theme::INK_FAINT,
            );
            painter.text(
                Pos2::new(chevron.right() + px(8.0), h.center().y),
                Align2::LEFT_CENTER,
                sub,
                crumb,
                theme::INK,
            );
        }
        let hint_right = Pos2::new(h.right() - px(16.0), h.center().y);
        let cap = kit::rail::kbd(ui, hint_right, "Ctrl Enter", s);
        ui.painter().text(
            Pos2::new(cap.left() - px(8.0), h.center().y),
            Align2::RIGHT_CENTER,
            "Quick build",
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_FAINT,
        );
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
        for (level, msg) in self.pipeline_ui.drain(ctx) {
            self.toasts.push(level, msg);
        }
        let pipeline_busy = self.pipeline_ui.running;
        for (level, msg) in self.scheduler.tick(&self.projects, pipeline_busy, ctx) {
            self.toasts.push(level, msg);
        }
        for msg in self.notify_ui.drain() {
            self.toasts.push(kit::ToastLevel::Error, msg);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            let lay = layout::compute(ui.max_rect());
            let s = lay.scale;

            ui.painter()
                .rect_filled(ui.max_rect(), theme::corner(0.0), theme::SIDEBAR);
            let radius = theme::px(theme::RADIUS_PANEL, s);
            theme::shadow(ui.painter(), lay.frame, radius, 18, 90);
            kit::bordered(
                ui.painter(),
                lay.frame,
                theme::SURFACE,
                theme::LINE_SOFT,
                radius,
            );

            let account = self
                .projects
                .get(self.build_ui.project_sel)
                .map(|p| {
                    if p.default_account.is_empty() {
                        p.steam_username.clone()
                    } else {
                        p.default_account.clone()
                    }
                })
                .unwrap_or_default();
            let avatar_letter = account
                .chars()
                .next()
                .map(|c| c.to_ascii_uppercase())
                .unwrap_or('S');
            let rail_state = kit::rail::RailState {
                active: self.page,
                sub_tabs: &self.sub_tabs,
                avatar: avatar_letter,
                account: &account,
            };
            match kit::rail(ui, lay.rail, rail_state, s) {
                Some(RailAction::Navigate(page)) => self.goto(&ctx, page),
                Some(RailAction::SubTab(page, tab)) => {
                    self.sub_tabs[nav::index(page)] = tab;
                    self.goto(&ctx, page);
                }
                Some(RailAction::Search) => self.palette.toggle(),
                Some(RailAction::Avatar) => self.account_menu = !self.account_menu,
                _ => {}
            }
            self.header_bar(ui, &lay);
            crate::ui::window::titlebar(ui, lay.titlebar, s);

            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
                self.palette.toggle();
            }
            if !self.palette.open
                && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter))
            {
                crate::ui::palette::start_build(self, false, &ctx);
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
                Pos2::new(lay.frame.left(), lay.header.bottom() + 1.0),
                lay.frame.max,
            );
            if let Some(tr) = self.transition {
                let t = tr.progress(&ctx);
                if let Some(prev) = self.prev_page {
                    self.render_page(ui, prev, &lay, clip, 1.0 - t);
                }
                self.render_page(ui, self.page, &lay, clip, t);
                if tr.is_done(&ctx) {
                    self.transition = None;
                    self.prev_page = None;
                }
            } else {
                self.render_page(ui, self.page, &lay, clip, 1.0);
            }

            crate::ui::dropimport::show(self, ui, lay.frame, s);
            crate::ui::palette::show(self, ui, s);
            crate::ui::accountmenu::show(self, ui, lay.rail, s);
            crate::ui::wizard::show(self, ui, s);
            crate::ui::quitguard::show(self, ui, s);

            if let Some(page) = self.nav_request.take() {
                self.goto(&ctx, page);
            }

            self.toasts.show(ui, lay.toast_anchor, s);
            crate::ui::window::resize_edges(ui, ui.max_rect(), s);
        });
    }
}
