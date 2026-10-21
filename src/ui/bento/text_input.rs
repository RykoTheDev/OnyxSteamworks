#![allow(dead_code)]

use eframe::egui::{
    self, Align2, FontId, Frame, Pos2, Rect, Response, Stroke, StrokeKind, TextEdit, Vec2,
};

use super::panel;
use crate::ui::theme;

pub struct TextInput<'a> {
    text: &'a mut String,
    hint: &'a str,
    error: bool,
    helper: Option<&'a str>,
    mask: bool,
}

impl<'a> TextInput<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            hint: "",
            error: false,
            helper: None,
            mask: false,
        }
    }

    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }

    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    pub fn helper(mut self, helper: &'a str) -> Self {
        self.helper = Some(helper);
        self
    }

    pub fn mask(mut self, mask: bool) -> Self {
        self.mask = mask;
        self
    }

    pub fn show(self, ui: &mut egui::Ui, rect: Rect, s: f32) -> Response {
        let px = |v: f32| theme::px(v, s);
        let id = ui.make_persistent_id(("text_input", rect.left() as i32, rect.top() as i32));
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let was = ctx.data_mut(|d| d.get_temp::<bool>(id.with("e")).unwrap_or(false));
        if self.error && !was {
            ctx.data_mut(|d| d.insert_temp(id.with("s"), now));
        }
        ctx.data_mut(|d| d.insert_temp(id.with("e"), self.error));
        let shake = if self.error {
            let start = ctx.data_mut(|d| d.get_temp::<f64>(id.with("s")).unwrap_or(now));
            let elapsed = (now - start) as f32;
            if elapsed < 0.6 {
                ctx.request_repaint();
                (-elapsed * 6.0).exp() * (elapsed * 42.0).sin() * px(4.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        let field = rect.translate(Vec2::new(shake, 0.0));

        panel::panel(&ui.painter().clone(), field, theme::PANEL_ALT, px(16.0));

        let inner = Rect::from_min_max(
            Pos2::new(field.left() + px(16.0), field.top()),
            Pos2::new(field.right() - px(16.0), field.bottom()),
        );
        let resp = ui.put(
            inner,
            TextEdit::singleline(self.text)
                .frame(Frame::NONE)
                .password(self.mask)
                .hint_text(self.hint)
                .desired_width(f32::INFINITY),
        );

        let outline = if self.error {
            Some(theme::DANGER)
        } else if resp.has_focus() {
            Some(theme::INK)
        } else {
            None
        };
        if let Some(c) = outline {
            ui.painter().rect_stroke(
                field,
                theme::corner(px(16.0)),
                Stroke::new(2.0, c),
                StrokeKind::Inside,
            );
        }

        if let Some(helper) = self.helper {
            let color = if self.error {
                theme::DANGER
            } else {
                theme::INK_MUTED
            };
            ui.painter().text(
                Pos2::new(field.left() + px(4.0), field.bottom() + px(4.0)),
                Align2::LEFT_TOP,
                helper,
                FontId::new(px(12.0), theme::family_medium()),
                color,
            );
        }
        resp
    }
}
