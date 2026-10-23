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
        let fill = theme::lerp_color(theme::SURFACE_2, theme::DANGER, flash * 0.12);

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
            let font = FontId::new(px(13.0), theme::family_medium());
            let text = tail_fit(painter, shown, &font, inner.width());
            let clipped = text != shown;
            painter.with_clip_rect(inner).text(
                Pos2::new(inner.left(), inner.center().y),
                Align2::LEFT_CENTER,
                text,
                font,
                color,
            );
            let resp = ui.interact(field, id.with("ro"), egui::Sense::hover());
            if clipped {
                resp.on_hover_text(shown)
            } else {
                resp
            }
        } else {
            let mut field_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
            ));
            field_ui.add(
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
            theme::ACCENT
        } else if resp.hovered() {
            theme::FIELD_HI
        } else {
            theme::LINE
        };
        let width = if self.error { 1.0 + flash * 0.75 } else { 1.0 };
        if resp.has_focus() && !self.error {
            ui.painter().rect_stroke(
                field.expand(2.0),
                theme::corner(radius + 2.0),
                Stroke::new(2.0, theme::with_alpha(theme::ACCENT, 60)),
                StrokeKind::Inside,
            );
        }
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

pub(crate) fn head_fit(painter: &egui::Painter, text: &str, font: &FontId, max_w: f32) -> String {
    let width = |t: &str| {
        painter
            .layout_no_wrap(t.to_owned(), font.clone(), theme::INK)
            .size()
            .x
    };
    if width(text) <= max_w {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let candidate: String = chars[..mid].iter().copied().chain(['\u{2026}']).collect();
        if width(&candidate) <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let kept: String = chars[..lo].iter().collect();
    format!("{}\u{2026}", kept.trim_end())
}

pub(crate) fn tail_fit(painter: &egui::Painter, text: &str, font: &FontId, max_w: f32) -> String {
    let width = |t: &str| {
        painter
            .layout_no_wrap(t.to_owned(), font.clone(), theme::INK)
            .size()
            .x
    };
    if width(text) <= max_w {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi) / 2;
        let candidate: String = std::iter::once('\u{2026}')
            .chain(chars[mid..].iter().copied())
            .collect();
        if width(&candidate) <= max_w {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    std::iter::once('\u{2026}')
        .chain(chars[lo..].iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use eframe::egui::{self, Pos2, Rect, Vec2};

    use super::{Input, head_fit, tail_fit};

    #[test]
    fn placing_an_input_above_the_cursor_does_not_move_it_back_up() {
        let ctx = egui::Context::default();
        let mut text = String::new();
        let mut before = 0.0;
        let mut after = 0.0;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let (card, _) = ui.allocate_exact_size(Vec2::new(300.0, 200.0), egui::Sense::hover());
            before = ui.cursor().top();
            let field =
                Rect::from_min_size(Pos2::new(card.left(), card.top()), Vec2::new(200.0, 30.0));
            Input::new(&mut text).show(ui, field, 1.0);
            after = ui.cursor().top();
        });
        output.textures_delta.clear();
        assert_eq!(before, after);
    }

    #[test]
    fn long_text_keeps_its_start_within_the_width() {
        let ctx = egui::Context::default();
        let mut fitted = String::new();
        let mut width = 0.0;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = egui::FontId::proportional(12.0);
            fitted = head_fit(
                ui.painter(),
                "When files change in D:\\Storage\\SteamSDK\\tools\\ContentBuilder\\content",
                &font,
                120.0,
            );
            width = ui
                .painter()
                .layout_no_wrap(fitted.clone(), font, egui::Color32::WHITE)
                .size()
                .x;
        });
        output.textures_delta.clear();
        assert!(fitted.starts_with("When files"));
        assert!(fitted.ends_with('\u{2026}'));
        assert!(width <= 120.0);
    }

    #[test]
    fn long_readonly_text_keeps_its_tail_within_the_width() {
        let ctx = egui::Context::default();
        let long = r"C:\Users\Ryko\AppData\Roaming\steampipe\SteamPipe GUI\data\steamcmd";
        let mut fitted = String::new();
        let mut short = String::new();
        let mut width = 0.0;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = egui::FontId::proportional(13.0);
            fitted = tail_fit(ui.painter(), long, &font, 160.0);
            short = tail_fit(ui.painter(), r"C:\tools", &font, 160.0);
            width = ui
                .painter()
                .layout_no_wrap(fitted.clone(), font, egui::Color32::WHITE)
                .size()
                .x;
        });
        output.textures_delta.clear();
        assert!(fitted.starts_with('\u{2026}'));
        assert!(fitted.ends_with(r"data\steamcmd"));
        assert!(width <= 160.0);
        assert_eq!(short, r"C:\tools");
    }
}
