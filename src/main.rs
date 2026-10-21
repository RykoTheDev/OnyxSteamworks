mod app;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Onyx")
            .with_inner_size([960.0, 540.0])
            .with_min_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native("Onyx", options, Box::new(|_cc| Ok(Box::new(app::OnyxApp))))
}
