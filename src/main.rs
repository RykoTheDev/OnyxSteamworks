#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod config;
mod features;
mod history;
mod model;
mod secrets;
mod steamcmd;
mod ui;
mod validate;
mod vdf;

use eframe::egui;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Onyx")
            .with_inner_size([1180.0, 830.0])
            .with_min_inner_size([900.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Onyx",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
