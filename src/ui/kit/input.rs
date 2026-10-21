use eframe::egui::{
    self, Align2, FontId, Frame, Pos2, Rect, Response, Stroke, StrokeKind, TextEdit,
};

use crate::ui::{anim, kit, theme};

pub struct Input<'a> {
    text: &'a mut String,
    hint: &'a str,
    error: bool,
    helper: Option<&'a str>,
    mask: bool,
    readonly: bool,
}

impl<'a> Input<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            hint: "",
            error: false,
            helper: None,
            mask: false,
            readonly: false,
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

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn show(self, ui: &mut egui::Ui, rect: Rect, s: f32) -> Response {
        let px = |v: f32| theme::px(v, s);
        let radius = px(theme::RADIUS_FIELD);
        let id = ui.make_persistent_id(("input", rect.left() as i32, rect.top() as i32));
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let was = ctx.data_mut(|d| d.get_temp::<bool>(id.with("e")).unwrap_or(false));
        if self.error && !was {
            ctx.data_mut(|d| d.insert_temp(id.with("s"), now));
        }
        ctx.data_mut(|d| d.insert_temp(id.with("e"), self.error));
        let flash = if self.error && !anim::reduce_motion() {
            let start = ctx.data_mut(|d| d.get_temp::<f64>(id.with("s")).unwrap_or(now));
            let elapsed = (now - start) as f32;
            if elapsed < 0.16 {
                ctx.request_repaint();
                1.0 - anim::ease_out_cubic(elapsed / 0.16)
            } else {
                0.0
            }
        } else {
            0.0
        };
        let field = rect;
        let fill = theme::lerp_color(theme::FIELD, theme::DANGER, flash * 0.12);

        kit::surface(&ui.painter().clone(), field, fill, radius);

        let inner = Rect::from_min_max(
            Pos2::new(field.left() + px(14.0), field.top()),
            Pos2::new(field.right() - px(14.0), field.bottom()),
        );

        let resp = if self.readonly {
            let painter = ui.painter();
            let shown = if self.text.is_empty() {
                self.hint
            } else {
                self.text.as_str()
            };
            let color = if self.text.is_empty() {
                theme::INK_FAINT
            } else {
                theme::INK_DIM
            };
            painter.text(
                Pos2::new(inner.left(), inner.center().y),
                Align2::LEFT_CENTER,
                shown,
                FontId::new(px(13.0), theme::family_medium()),
                color,
            );
            ui.interact(field, id.with("ro"), egui::Sense::hover())
        } else {
            ui.put(
                inner,
                TextEdit::singleline(self.text)
                    .frame(Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .align(Align2::LEFT_CENTER)
                    .password(self.mask)
                    .hint_text(self.hint)
                    .text_color(theme::INK)
                    .desired_width(f32::INFINITY),
            )
        };

        let border = if self.error {
            theme::DANGER
        } else if resp.has_focus() {
            theme::INK_DIM
        } else {
            theme::LINE
        };
        let width = if self.error {
            1.5 + flash * 0.75
        } else if resp.has_focus() {
            1.5
        } else {
            1.0
        };
        ui.painter().rect_stroke(
            field,
            theme::corner(radius),
            Stroke::new(width, border),
            StrokeKind::Inside,
        );

        if let Some(helper) = self.helper {
            let color = if self.error {
                theme::DANGER
            } else {
                theme::INK_MUTED
            };
            ui.painter().text(
                Pos2::new(field.left() + px(2.0), field.bottom() + px(5.0)),
                Align2::LEFT_TOP,
                helper,
                FontId::new(px(11.5), theme::family_medium()),
                color,
            );
        }
        resp
    }
}
