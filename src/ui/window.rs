use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, ResizeDirection, Sense, Stroke, Vec2,
    ViewportCommand,
};

use crate::ui::{anim, theme};

pub const TITLEBAR_H: f32 = 40.0;
const BUTTON_W: f32 = 46.0;
const EDGE: f32 = 6.0;
const CLOSE_RED: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

#[derive(Clone, Copy)]
enum Control {
    Minimize,
    Maximize,
    Close,
}

pub fn round_corners(cc: &eframe::CreationContext<'_>) {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::Graphics::Dwm::{
            DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
        };
        let Ok(handle) = cc.window_handle() else {
            return;
        };
        if let RawWindowHandle::Win32(win) = handle.as_raw() {
            let preference = DWMWCP_ROUND;
            unsafe {
                DwmSetWindowAttribute(
                    win.hwnd.get() as _,
                    DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                    &preference as *const _ as *const core::ffi::c_void,
                    std::mem::size_of_val(&preference) as u32,
                );
            }
        }
    }
    #[cfg(not(windows))]
    let _ = cc;
}

pub fn titlebar(ui: &mut egui::Ui, rect: Rect, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let ctx = ui.ctx().clone();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

    let drag = ui.interact(
        rect,
        ui.make_persistent_id("titlebar_drag"),
        Sense::click_and_drag(),
    );
    if drag.double_clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if drag.drag_started_by(egui::PointerButton::Primary) {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let mark = Rect::from_center_size(
        Pos2::new(rect.left() + px(28.0), rect.center().y),
        Vec2::splat(px(20.0)),
    );
    ui.painter()
        .rect_filled(mark, theme::corner(px(5.0)), theme::ACCENT);
    ui.painter().text(
        mark.center(),
        Align2::CENTER_CENTER,
        theme::icon::SPARKLE.to_string(),
        theme::icon_font(px(12.0), true),
        theme::PRIMARY_INK,
    );
    ui.painter().text(
        Pos2::new(mark.right() + px(10.0), rect.center().y),
        Align2::LEFT_CENTER,
        "Onyx",
        FontId::new(px(13.5), theme::family_semibold()),
        theme::INK,
    );

    let w = px(BUTTON_W);
    let mut x = rect.right() - w * 3.0;
    for control in [Control::Minimize, Control::Maximize, Control::Close] {
        let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, rect.height()));
        let id = ui.make_persistent_id(("window_control", control as u8));
        let resp = ui.interact(r, id, Sense::click());
        let hover = anim::anim_bool(
            &ctx,
            id.with("h"),
            resp.hovered(),
            0.1,
            anim::ease_out_cubic,
        );
        let pressed = resp.is_pointer_button_down_on();
        let (fill, ink) = match control {
            Control::Close => (
                theme::with_alpha(
                    CLOSE_RED,
                    (hover * if pressed { 200.0 } else { 255.0 }) as u8,
                ),
                theme::lerp_color(theme::INK_MUTED, Color32::WHITE, hover),
            ),
            _ => (
                theme::with_alpha(theme::FIELD_HI, (hover * 255.0) as u8),
                theme::lerp_color(theme::INK_MUTED, theme::INK, hover),
            ),
        };
        ui.painter().rect_filled(r, theme::corner(0.0), fill);
        draw_glyph(ui.painter(), r.center(), control, maximized, ink, s);
        let tip = match control {
            Control::Minimize => "Minimize",
            Control::Maximize if maximized => "Restore",
            Control::Maximize => "Maximize",
            Control::Close => "Close",
        };
        if resp.on_hover_text(tip).clicked() {
            match control {
                Control::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                Control::Maximize => ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized)),
                Control::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
        }
        x += w;
    }
}

fn draw_glyph(
    painter: &egui::Painter,
    c: Pos2,
    control: Control,
    maximized: bool,
    ink: Color32,
    s: f32,
) {
    let px = |v: f32| theme::px(v, s);
    let stroke = Stroke::new(1.0, ink);
    let half = px(5.0);
    let c = Pos2::new(c.x.round() + 0.5, c.y.round() + 0.5);
    match control {
        Control::Minimize => {
            painter.line_segment(
                [Pos2::new(c.x - half, c.y), Pos2::new(c.x + half, c.y)],
                stroke,
            );
        }
        Control::Maximize if maximized => {
            let back = Rect::from_min_max(
                Pos2::new(c.x - half + px(2.0), c.y - half),
                Pos2::new(c.x + half, c.y + half - px(2.0)),
            );
            let front = Rect::from_min_max(
                Pos2::new(c.x - half, c.y - half + px(2.0)),
                Pos2::new(c.x + half - px(2.0), c.y + half),
            );
            painter.line_segment([back.left_top(), back.right_top()], stroke);
            painter.line_segment([back.right_top(), back.right_bottom()], stroke);
            painter.rect_stroke(
                front,
                theme::corner(px(1.0)),
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        Control::Maximize => {
            let r = Rect::from_center_size(c, Vec2::splat(half * 2.0));
            painter.rect_stroke(r, theme::corner(px(1.5)), stroke, egui::StrokeKind::Middle);
        }
        Control::Close => {
            painter.line_segment(
                [
                    Pos2::new(c.x - half, c.y - half),
                    Pos2::new(c.x + half, c.y + half),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + half, c.y - half),
                    Pos2::new(c.x - half, c.y + half),
                ],
                stroke,
            );
        }
    }
}

pub fn resize_edges(ui: &mut egui::Ui, window: Rect, s: f32) {
    let ctx = ui.ctx().clone();
    if ctx.input(|i| {
        i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
    }) {
        return;
    }
    let e = theme::px(EDGE, s).max(4.0);
    let c = e * 2.0;
    let (l, r, t, b) = (window.left(), window.right(), window.top(), window.bottom());
    let zones = [
        (
            Rect::from_min_max(Pos2::new(l, t), Pos2::new(l + c, t + c)),
            ResizeDirection::NorthWest,
            CursorIcon::ResizeNorthWest,
        ),
        (
            Rect::from_min_max(Pos2::new(r - c, t), Pos2::new(r, t + c)),
            ResizeDirection::NorthEast,
            CursorIcon::ResizeNorthEast,
        ),
        (
            Rect::from_min_max(Pos2::new(l, b - c), Pos2::new(l + c, b)),
            ResizeDirection::SouthWest,
            CursorIcon::ResizeSouthWest,
        ),
        (
            Rect::from_min_max(Pos2::new(r - c, b - c), Pos2::new(r, b)),
            ResizeDirection::SouthEast,
            CursorIcon::ResizeSouthEast,
        ),
        (
            Rect::from_min_max(Pos2::new(l + c, t), Pos2::new(r - c, t + e)),
            ResizeDirection::North,
            CursorIcon::ResizeVertical,
        ),
        (
            Rect::from_min_max(Pos2::new(l + c, b - e), Pos2::new(r - c, b)),
            ResizeDirection::South,
            CursorIcon::ResizeVertical,
        ),
        (
            Rect::from_min_max(Pos2::new(l, t + c), Pos2::new(l + e, b - c)),
            ResizeDirection::West,
            CursorIcon::ResizeHorizontal,
        ),
        (
            Rect::from_min_max(Pos2::new(r - e, t + c), Pos2::new(r, b - c)),
            ResizeDirection::East,
            CursorIcon::ResizeHorizontal,
        ),
    ];
    for (i, (zone, dir, cursor)) in zones.into_iter().enumerate() {
        let resp = ui.interact(
            zone,
            ui.make_persistent_id(("resize_edge", i)),
            Sense::drag(),
        );
        if resp.hovered() || resp.dragged() {
            ctx.set_cursor_icon(cursor);
        }
        if resp.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
        }
    }
}
