#![allow(dead_code)]

use eframe::egui::{
    self, Align2, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Vec2, epaint::Shape,
};

use crate::ui::{anim, theme};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PillKind {
    Primary,
    Secondary,
    Danger,
}

pub struct PillButton<'a> {
    label: &'a str,
    kind: PillKind,
    icon: Option<char>,
    loading: bool,
    enabled: bool,
    reason: Option<&'a str>,
}

impl<'a> PillButton<'a> {
    pub fn new(label: &'a str, kind: PillKind) -> Self {
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
        let id = ui.make_persistent_id(("pill", self.label, rect.left() as i32, rect.top() as i32));
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
            0.14,
            anim::ease_out_cubic,
        );
        let press = anim::anim_bool(
            &ctx,
            id.with("p"),
            resp.is_pointer_button_down_on(),
            0.09,
            anim::ease_out_cubic,
        );
        let scale = 1.0 + 0.03 * hover - 0.06 * press;
        let r = anim::scale_rect(rect, scale);
        let radius = r.height() / 2.0;

        let (fill, fg, outline) = match self.kind {
            PillKind::Primary => (theme::INK, theme::PANEL, None),
            PillKind::Secondary => (theme::PANEL, theme::INK, Some(theme::INK)),
            PillKind::Danger => (theme::PANEL, theme::DANGER, Some(theme::DANGER)),
        };
        let dim = if self.enabled { 1.0 } else { 0.4 };
        let painter = ui.painter();
        painter.rect_filled(r, theme::corner(radius), anim::with_opacity(fill, dim));
        if let Some(c) = outline {
            painter.rect_stroke(
                r,
                theme::corner(radius),
                Stroke::new(1.0, anim::with_opacity(c, dim)),
                StrokeKind::Inside,
            );
        }

        if self.loading {
            spinner(painter, r.center(), px(9.0), fg, &ctx);
        } else {
            let fid = FontId::new(px(14.0), theme::family_medium());
            let color = anim::with_opacity(fg, dim);
            let label_w = painter
                .layout_no_wrap(self.label.to_string(), fid.clone(), color)
                .size()
                .x;
            if let Some(glyph) = self.icon {
                let icon_w = px(16.0);
                let gap = px(6.0);
                let start = r.center().x - (icon_w + gap + label_w) / 2.0;
                painter.text(
                    Pos2::new(start + icon_w / 2.0, r.center().y),
                    Align2::CENTER_CENTER,
                    glyph.to_string(),
                    theme::icon_font(icon_w, false),
                    color,
                );
                painter.text(
                    Pos2::new(start + icon_w + gap, r.center().y),
                    Align2::LEFT_CENTER,
                    self.label,
                    fid,
                    color,
                );
            } else {
                painter.text(r.center(), Align2::CENTER_CENTER, self.label, fid, color);
            }
        }

        if resp.has_focus() {
            let ring = if self.kind == PillKind::Primary {
                theme::PANEL
            } else {
                theme::INK
            };
            theme::focus_ring(painter, r.expand(px(3.0)), radius + px(3.0), ring);
        }

        if !self.enabled
            && let Some(reason) = self.reason
        {
            return resp.on_hover_ui(|ui| {
                ui.colored_label(theme::PANEL, reason);
            });
        }
        resp
    }
}

fn spinner(
    painter: &egui::Painter,
    center: Pos2,
    r: f32,
    color: egui::Color32,
    ctx: &egui::Context,
) {
    ctx.request_repaint();
    let t = ctx.input(|i| i.time) as f32;
    let start = t * 5.0;
    let sweep = 4.6;
    let n = 24;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| center + Vec2::angled(start + sweep * (i as f32 / n as f32)) * r)
        .collect();
    painter.add(Shape::line(pts, Stroke::new(2.0, color)));
}
