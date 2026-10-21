use eframe::egui;

use crate::app::App;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("SteamPipe GUI");
    ui.label("Debug build. Projects loaded from the config directory:");
    ui.separator();

    if app.projects.is_empty() {
        ui.label("(no projects yet)");
        return;
    }

    for project in &app.projects {
        let app_id = if project.app_id.is_empty() {
            "unset"
        } else {
            project.app_id.as_str()
        };
        ui.label(format!(
            "{}  -  app_id {}  -  {} depot(s)  -  id {}",
            project.name,
            app_id,
            project.depots.len(),
            project.id,
        ));
    }
}
