#![allow(dead_code)]

use eframe::egui::{
    self, Align2, FontId, Pos2, Rect, Vec2, emath::Rot2, epaint::Mesh, epaint::TextShape,
};

use crate::ui::theme;

const ANGLES: [f32; 4] = [-0.314, -0.070, 0.175, 0.420];

pub fn fan_cards(ui: &mut egui::Ui, parent: Rect, letters: &[char], s: f32) {
    let px = |v: f32| theme::px(v, s);
    let cw = px(140.0);
    let ch = px(180.0);
    let painter = ui.painter_at(parent);
    let anchor = Pos2::new(parent.right() - cw * 0.55, parent.bottom() - ch * 0.35);
    for (i, letter) in letters.iter().enumerate().take(ANGLES.len()) {
        let angle = ANGLES[i];
        let center = Pos2::new(
            anchor.x - i as f32 * px(10.0),
            anchor.y - i as f32 * px(6.0),
        );
        let card = Rect::from_center_size(center, Vec2::new(cw, ch));
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(card, theme::PANEL_ALT);
        mesh.rotate(Rot2::from_angle(angle), center);
        painter.add(mesh);
        let galley = painter.layout_no_wrap(
            letter.to_uppercase().to_string(),
            FontId::new(px(22.0), theme::family_semibold()),
            theme::INK_MUTED,
        );
        painter.add(
            TextShape::new(center, galley, theme::INK_MUTED)
                .with_angle_and_anchor(angle, Align2::CENTER_CENTER),
        );
    }
}
