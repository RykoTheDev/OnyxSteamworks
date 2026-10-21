use eframe::egui;

use crate::app::App;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let s = theme::scale(ui.available_size());
    egui::Frame::new()
        .fill(theme::PANEL)
        .corner_radius(theme::corner(theme::px(theme::RADIUS_CARD, s)))
        .inner_margin(theme::px(theme::SPACE_LG, s))
        .show(ui, |ui| {
            ui.visuals_mut().override_text_color = Some(theme::INK);
            ui.heading("SteamPipe GUI");
            ui.label("Debug build. Projects loaded from the config directory:");
            ui.separator();

            if app.projects.is_empty() {
                ui.colored_label(theme::INK_MUTED, "(no projects yet)");
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
        });
}
