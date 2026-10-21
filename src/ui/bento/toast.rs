#![allow(dead_code)]

use eframe::egui::{
    self, Align2, Color32, FontId, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};

use crate::ui::{anim, theme};

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

        let w = px(280.0);
        let h = px(48.0);
        let gap = px(10.0);
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
                    ui.painter()
                        .rect_filled(r, theme::corner(h / 2.0), theme::INK);
                    ui.painter().rect_stroke(
                        r,
                        theme::corner(h / 2.0),
                        Stroke::new(1.0, theme::with_alpha(theme::PANEL, 51)),
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
                        theme::PANEL,
                    );
                    let remaining = 1.0 - (age / ttl as f32).clamp(0.0, 1.0);
                    let lw = (w - px(28.0)) * remaining;
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            Pos2::new(r.left() + px(14.0), r.bottom() - px(7.0)),
                            Vec2::new(lw, px(2.5)),
                        ),
                        theme::corner(px(1.5)),
                        t.level.color(),
                    );
                    y += h + gap;
                }
                let _ = Sense::hover();
            });
        area.response
    }
}
