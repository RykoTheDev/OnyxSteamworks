use eframe::egui::{self, Align2, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind};

use crate::ui::{anim, theme};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

pub struct Button<'a> {
    label: &'a str,
    kind: ButtonKind,
    icon: Option<char>,
    loading: bool,
    enabled: bool,
    reason: Option<&'a str>,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str, kind: ButtonKind) -> Self {
        Self {
            label,
            kind,
            icon: None,
            loading: false,
            enabled: true,
            reason: None,
        }
    }

    pub fn icon(mut self, glyph: char) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn disabled_reason(mut self, reason: &'a str) -> Self {
        self.reason = Some(reason);
        self
    }

    pub fn show(self, ui: &mut egui::Ui, rect: Rect, s: f32) -> Response {
        let px = |v: f32| theme::px(v, s);
        let id = ui.make_persistent_id(("btn", self.label, rect.left() as i32, rect.top() as i32));
        let sense = if self.enabled && !self.loading {
            Sense::click()
        } else {
            Sense::hover()
        };
        let resp = ui.interact(rect, id, sense);
        let ctx = ui.ctx().clone();
        let hover = anim::anim_bool(
            &ctx,
            id.with("h"),
            resp.hovered() && self.enabled,
            0.12,
            anim::ease_out_cubic,
        );
        let press = anim::anim_bool(
            &ctx,
            id.with("p"),
            resp.is_pointer_button_down_on(),
            0.08,
            anim::ease_out_cubic,
        );
        let radius = px(theme::RADIUS_FIELD);

        let (base_fill, hi_fill, fg, border) = match self.kind {
            ButtonKind::Primary => (theme::PRIMARY, theme::INK, theme::PRIMARY_INK, None),
            ButtonKind::Secondary => (theme::FIELD, theme::FIELD_HI, theme::INK, Some(theme::LINE)),
            ButtonKind::Danger => (
                theme::FIELD,
                theme::FIELD_HI,
                theme::DANGER,
                Some(theme::DANGER),
            ),
            ButtonKind::Ghost => (theme::SURFACE, theme::FIELD, theme::INK_DIM, None),
        };
        let dim = if self.enabled { 1.0 } else { 0.4 };
        let fill = theme::lerp_color(base_fill, hi_fill, hover * 0.6 + press * 0.4);
        let painter = ui.painter();
        painter.rect_filled(rect, theme::corner(radius), anim::with_opacity(fill, dim));
        if let Some(c) = border {
            painter.rect_stroke(
                rect,
                theme::corner(radius),
                Stroke::new(1.0, anim::with_opacity(c, dim)),
                StrokeKind::Inside,
            );
        }

        let color = anim::with_opacity(fg, dim);
        if self.loading {
            crate::ui::kit::spinner(painter, rect.center(), px(8.0), fg, &ctx);
        } else {
            let fid = FontId::new(px(13.0), theme::family_semibold());
            let label_w = painter
                .layout_no_wrap(self.label.to_string(), fid.clone(), color)
                .size()
                .x;
            if let Some(glyph) = self.icon {
                let icon_w = px(15.0);
                let gap = px(7.0);
                let start = rect.center().x - (icon_w + gap + label_w) / 2.0;
                painter.text(
                    Pos2::new(start + icon_w / 2.0, rect.center().y),
                    Align2::CENTER_CENTER,
                    glyph.to_string(),
                    theme::icon_font(icon_w, false),
                    color,
                );
                painter.text(
                    Pos2::new(start + icon_w + gap, rect.center().y),
                    Align2::LEFT_CENTER,
                    self.label,
                    fid,
                    color,
                );
            } else {
                painter.text(rect.center(), Align2::CENTER_CENTER, self.label, fid, color);
            }
        }

        if resp.has_focus() {
            theme::focus_ring(painter, rect, radius, theme::FOCUS);
        }
        if !self.enabled
            && let Some(reason) = self.reason
        {
            return resp.on_hover_ui(|ui| {
                ui.colored_label(theme::INK, reason);
            });
        }
        resp
    }
}

pub fn icon_button(ui: &mut egui::Ui, rect: Rect, glyph: char, s: f32) -> Response {
    let px = |v: f32| theme::px(v, s);
    let id = ui.make_persistent_id(("iconbtn", glyph, rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
    let ctx = ui.ctx().clone();
    let hover = anim::anim_bool(
        &ctx,
        id.with("h"),
        resp.hovered(),
        0.12,
        anim::ease_out_cubic,
    );
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        theme::corner(px(theme::RADIUS_FIELD)),
        theme::lerp_color(theme::SURFACE, theme::FIELD, hover),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph.to_string(),
        theme::icon_font(px(16.0), false),
        theme::lerp_color(theme::INK_MUTED, theme::INK, hover),
    );
    resp
}
