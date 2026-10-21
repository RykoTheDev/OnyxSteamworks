#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod config;
mod features;
mod history;
mod keepawake;
mod kv;
mod model;
mod secrets;
mod steamcmd;
mod steamcmd_patterns;
mod ui;
mod validate;
mod vdf;

use eframe::egui;

fn main() -> eframe::Result {
    if std::env::args_os().count() > 1 {
        std::process::exit(cli::run());
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SteamPipe GUI")
            .with_inner_size([1180.0, 830.0])
            .with_min_inner_size([900.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SteamPipe GUI",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
