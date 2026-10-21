use eframe::egui::{self, Align2, Color32, FontId, Order, Pos2, Rect, Sense, Stroke, StrokeKind, UiBuilder, Vec2};

use crate::ui::{anim, theme};

pub fn modal(
    ui: &mut egui::Ui,
    id: egui::Id,
    open: &mut bool,
    size: Vec2,
    s: f32,
    add: impl FnOnce(&mut egui::Ui) -> bool,
) -> egui::Response {
    let ctx = ui.ctx().clone();
    let t = anim::anim_bool(&ctx, id.with("m"), *open, 0.2, anim::ease_out_back);
    if !*open && t <= 0.001 {
        return ui.interact(Rect::ZERO, id.with("closed"), Sense::hover());
    }
    let screen = ctx.content_rect();
    let area = egui::Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(screen.min)
        .show(&ctx, |ui| {
            let dim = theme::with_alpha(Color32::BLACK, (150.0 * t.clamp(0.0, 1.0)) as u8);
            ui.painter().rect_filled(screen, theme::corner(0.0), dim);
            if ui.interact(screen, id.with("dim"), Sense::click()).clicked() {
                *open = false;
            }
            let scale = 0.94 + 0.06 * t;
            let prect = Rect::from_center_size(screen.center(), size * scale);
            ui.painter().rect_filled(prect, theme::corner(theme::px(theme::RADIUS_PANEL_MIN, s)), theme::SURFACE);
            ui.painter().rect_stroke(
                prect,
                theme::corner(theme::px(theme::RADIUS_PANEL_MIN, s)),
                Stroke::new(1.0, theme::LINE),
                StrokeKind::Inside,
            );
            let mut cu = ui.new_child(UiBuilder::new().max_rect(prect));
            cu.set_opacity(t.clamp(0.0, 1.0));
            if add(&mut cu) {
                *open = false;
            }
        });
    area.response
}

pub fn tooltip(response: egui::Response, text: &str) -> egui::Response {
    let text = text.to_owned();
    response.on_hover_ui(|ui| {
        ui.colored_label(theme::INK, text);
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Success,
    Warning,
    Error,
}

impl ToastLevel {
    fn color(self) -> Color32 {
        match self {
            ToastLevel::Success => theme::SUCCESS,
            ToastLevel::Warning => theme::WARNING,
            ToastLevel::Error => theme::DANGER,
        }
    }
}

struct Toast {
    level: ToastLevel,
    text: String,
    born: f64,
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn push(&mut self, level: ToastLevel, text: impl Into<String>) {
        self.items.push(Toast {
            level,
            text: text.into(),
            born: -1.0,
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, anchor: Pos2, s: f32) -> egui::Response {
        let px = |v: f32| theme::px(v, s);
        let ttl = 3.5f64;
        let now = ui.input(|i| i.time);
        for t in &mut self.items {
            if t.born < 0.0 {
                t.born = now;
            }
        }
        self.items.retain(|t| now - t.born < ttl);
        let ctx = ui.ctx().clone();
        if !self.items.is_empty() {
            ctx.request_repaint();
        }
        let w = px(288.0);
        let h = px(46.0);
        let gap = px(10.0);
        let radius = px(theme::RADIUS_FIELD);
        let items = &self.items;
        let area = egui::Area::new(ui.make_persistent_id("toasts"))
            .order(Order::Foreground)
            .fixed_pos(Pos2::new(anchor.x - w, anchor.y))
            .show(&ctx, |ui| {
                let mut y = anchor.y;
                for t in items.iter() {
                    let age = (now - t.born) as f32;
                    let slide = anim::ease_out_cubic((age / 0.25).min(1.0));
                    let x = anchor.x - w + (1.0 - slide) * px(40.0);
                    let r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                    ui.painter().rect_filled(r, theme::corner(radius), theme::SURFACE_2);
                    ui.painter().rect_stroke(
                        r,
                        theme::corner(radius),
                        Stroke::new(1.0, theme::LINE),
                        StrokeKind::Inside,
                    );
                    ui.painter().circle_filled(
                        Pos2::new(r.left() + px(20.0), r.center().y),
                        px(4.0),
                        t.level.color(),
                    );
                    ui.painter().text(
                        Pos2::new(r.left() + px(34.0), r.center().y),
                        Align2::LEFT_CENTER,
                        &t.text,
                        FontId::new(px(13.0), theme::family_medium()),
                        theme::INK,
                    );
                    let remaining = 1.0 - (age / ttl as f32).clamp(0.0, 1.0);
                    let lw = (w - px(28.0)) * remaining;
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            Pos2::new(r.left() + px(14.0), r.bottom() - px(6.0)),
                            Vec2::new(lw, px(2.0)),
                        ),
                        theme::corner(px(1.0)),
                        theme::with_alpha(t.level.color(), 160),
                    );
                    y += h + gap;
                }
                let _ = Sense::hover();
            });
        area.response
    }
}
