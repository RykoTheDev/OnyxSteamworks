use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Vec2};

use super::chrome::{mb, para};
use super::{Intent, View};
use crate::secrets::BUILD_ACCOUNT_NOTE;
use crate::ui::kit::{self, Button, ButtonKind, Input};
use crate::ui::theme;

pub(super) fn title(step: usize) -> &'static str {
    match step {
        0 => "Install SteamCMD",
        1 => "Create your first project",
        2 => "Authorize your Steam account",
        _ => "You're ready to build",
    }
}

pub(super) fn body(ui: &mut egui::Ui, rect: Rect, view: &View, intent: &mut Intent, s: f32) {
    match view.step {
        0 => install(ui, rect, view, intent, s),
        1 => project(ui, rect, view, intent, s),
        2 => authorize(ui, rect, view, intent, s),
        _ => finish(ui, rect, view, s),
    }
}

fn install(ui: &mut egui::Ui, rect: Rect, view: &View, intent: &mut Intent, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let x = rect.left();
    let w = rect.width();
    let mut y = rect.top();
    y += para(
        ui,
        Pos2::new(x, y),
        w,
        "Onyx uploads builds with Valve's SteamCMD. Download it now, or point Onyx at an existing copy.",
        theme::INK_DIM,
        s,
    );
    y += px(18.0);

    label(ui, Pos2::new(x, y), "Install folder", s);
    let field = Rect::from_min_size(
        Pos2::new(x, y + px(18.0)),
        Vec2::new(w - px(132.0), px(36.0)),
    );
    let mut shown = view.install_dir.clone();
    Input::new(&mut shown)
        .hint("A default folder is used if left blank")
        .readonly(true)
        .show(ui, field, s);
    let browse = Rect::from_min_size(
        Pos2::new(field.right() + px(10.0), field.top()),
        Vec2::new(px(122.0), px(36.0)),
    );
    if Button::new("Browse", ButtonKind::Secondary)
        .icon(theme::icon::FOLDER)
        .show(ui, browse, s)
        .clicked()
    {
        intent.pick_install = true;
    }
    y = field.bottom() + px(24.0);

    if let Some(path) = &view.installed {
        check_line(ui, Pos2::new(x, y), true, "SteamCMD is ready", s);
        para(ui, Pos2::new(x, y + px(24.0)), w, path, theme::INK_MUTED, s);
    } else if view.extracting {
        kit::spinner(
            &ui.painter().clone(),
            Pos2::new(x + px(8.0), y + px(8.0)),
            px(8.0),
            theme::INK,
            ui.ctx(),
        );
        ui.painter().text(
            Pos2::new(x + px(26.0), y + px(8.0)),
            Align2::LEFT_CENTER,
            "Extracting SteamCMD…",
            FontId::new(px(13.0), theme::family_medium()),
            theme::INK_DIM,
        );
    } else if view.downloading {
        let bar = Rect::from_min_size(Pos2::new(x, y + px(4.0)), Vec2::new(w, px(8.0)));
        let frac = if view.dl_total > 0 {
            view.dl_done as f32 / view.dl_total as f32
        } else {
            0.0
        };
        kit::progress_bar(ui, bar, frac, s);
        let note = if view.dl_total > 0 {
            format!("Downloading… {} / {}", mb(view.dl_done), mb(view.dl_total))
        } else {
            format!("Downloading… {}", mb(view.dl_done))
        };
        ui.painter().text(
            Pos2::new(x, bar.bottom() + px(10.0)),
            Align2::LEFT_TOP,
            note,
            FontId::new(px(12.0), theme::family_medium()),
            theme::INK_MUTED,
        );
    } else {
        let btn = Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(200.0), px(40.0)));
        let label = if view.dl_error.is_some() {
            "Try again"
        } else {
            "Download SteamCMD"
        };
        if Button::new(label, ButtonKind::Primary)
            .icon(theme::icon::DOWNLOAD)
            .show(ui, btn, s)
            .clicked()
        {
            intent.start_install = true;
        }
    }
    if let Some(err) = &view.dl_error {
        para(
            ui,
            Pos2::new(x, rect.bottom() - px(40.0)),
            w,
            &format!("Could not install: {err}"),
            theme::DANGER,
            s,
        );
    }
}

fn project(ui: &mut egui::Ui, rect: Rect, view: &View, intent: &mut Intent, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let x = rect.left();
    let w = rect.width();
    let mut y = rect.top();
    y += para(
        ui,
        Pos2::new(x, y),
        w,
        "Point Onyx at your game's build folder. It detects the engine and prepares depots automatically.",
        theme::INK_DIM,
        s,
    );
    y += px(18.0);

    label(ui, Pos2::new(x, y), "Game build folder", s);
    let field = Rect::from_min_size(
        Pos2::new(x, y + px(18.0)),
        Vec2::new(w - px(162.0), px(36.0)),
    );
    let mut shown = view.game_dir.clone();
    Input::new(&mut shown)
        .hint("No folder chosen")
        .readonly(true)
        .show(ui, field, s);
    let browse = Rect::from_min_size(
        Pos2::new(field.right() + px(10.0), field.top()),
        Vec2::new(px(152.0), px(36.0)),
    );
    if Button::new("Choose folder", ButtonKind::Secondary)
        .icon(theme::icon::FOLDER)
        .show(ui, browse, s)
        .clicked()
    {
        intent.pick_game = true;
    }
    y = field.bottom() + px(22.0);

    if view.detecting {
        kit::spinner(
            &ui.painter().clone(),
            Pos2::new(x + px(8.0), y + px(8.0)),
            px(8.0),
            theme::INK,
            ui.ctx(),
        );
        ui.painter().text(
            Pos2::new(x + px(26.0), y + px(8.0)),
            Align2::LEFT_CENTER,
            "Scanning folder…",
            FontId::new(px(13.0), theme::family_medium()),
            theme::INK_DIM,
        );
        return;
    }
    let Some((name, engine, depots)) = &view.detected else {
        para(
            ui,
            Pos2::new(x, y),
            w,
            "Choose a folder to let Onyx inspect it.",
            theme::INK_MUTED,
            s,
        );
        return;
    };

    let card = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, px(96.0)));
    kit::bordered(
        &ui.painter().clone(),
        card,
        theme::SURFACE_2,
        theme::LINE,
        px(theme::RADIUS_FIELD),
    );
    let lines = [
        ("Name", name.clone()),
        ("Engine", engine.clone()),
        ("Depots", depots.to_string()),
    ];
    for (i, (k, v)) in lines.iter().enumerate() {
        let ly = card.top() + px(18.0) + i as f32 * px(21.0);
        ui.painter().text(
            Pos2::new(card.left() + px(16.0), ly),
            Align2::LEFT_TOP,
            *k,
            FontId::new(px(11.0), theme::family_label()),
            theme::INK_MUTED,
        );
        ui.painter().text(
            Pos2::new(card.left() + px(110.0), ly),
            Align2::LEFT_TOP,
            v,
            FontId::new(px(12.5), theme::family_medium()),
            theme::INK_DIM,
        );
    }
    y = card.bottom() + px(18.0);

    if let Some(created) = &view.created {
        check_line(ui, Pos2::new(x, y), true, &format!("Created {created}"), s);
    } else {
        let btn = Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(180.0), px(40.0)));
        if Button::new("Create project", ButtonKind::Primary)
            .icon(theme::icon::PLUS)
            .show(ui, btn, s)
            .clicked()
        {
            intent.create_project = true;
        }
    }
}

fn authorize(ui: &mut egui::Ui, rect: Rect, view: &View, intent: &mut Intent, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let x = rect.left();
    let w = rect.width();
    let mut y = rect.top();
    y += para(
        ui,
        Pos2::new(x, y),
        w,
        BUILD_ACCOUNT_NOTE,
        theme::INK_DIM,
        s,
    );
    y += px(20.0);

    label(ui, Pos2::new(x, y), "Account", s);
    let field = Rect::from_min_size(Pos2::new(x, y + px(18.0)), Vec2::new(w, px(36.0)));
    kit::bordered(
        &ui.painter().clone(),
        field,
        theme::FIELD,
        theme::LINE,
        px(theme::RADIUS_FIELD),
    );
    ui.painter().text(
        Pos2::new(field.left() + px(14.0), field.center().y),
        Align2::LEFT_CENTER,
        &view.account,
        FontId::new(px(13.0), theme::family_medium()),
        theme::INK_DIM,
    );
    y = field.bottom() + px(22.0);

    if view.authorized {
        check_line(
            ui,
            Pos2::new(x, y),
            true,
            "Authorizing on the Build page",
            s,
        );
    } else {
        let btn = Rect::from_min_size(Pos2::new(x, y), Vec2::new(px(200.0), px(40.0)));
        if Button::new("Authorize account", ButtonKind::Primary)
            .icon(theme::icon::CHECK)
            .show(ui, btn, s)
            .clicked()
        {
            intent.authorize = true;
        }
        para(
            ui,
            Pos2::new(x, btn.bottom() + px(14.0)),
            w,
            "Opens the Build page so you can answer any Steam Guard prompt. SteamCMD caches the login for later builds.",
            theme::INK_MUTED,
            s,
        );
    }
}

fn finish(ui: &mut egui::Ui, rect: Rect, view: &View, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let x = rect.left();
    let w = rect.width();
    let mut y = rect.top();
    y += para(
        ui,
        Pos2::new(x, y),
        w,
        "You're set. We recommend a preview build first — it validates the whole setup with Steam without publishing anything to players.",
        theme::INK_DIM,
        s,
    );
    y += px(22.0);

    check_line(
        ui,
        Pos2::new(x, y),
        view.installed.is_some(),
        "SteamCMD installed",
        s,
    );
    check_line(
        ui,
        Pos2::new(x, y + px(30.0)),
        view.created.is_some(),
        "First project created",
        s,
    );
    check_line(
        ui,
        Pos2::new(x, y + px(60.0)),
        view.authorized,
        "Account authorized",
        s,
    );
}

fn label(ui: &egui::Ui, pos: Pos2, text: &str, s: f32) {
    ui.painter().text(
        pos,
        Align2::LEFT_TOP,
        text,
        FontId::new(theme::px(11.5, s), theme::family_label()),
        theme::INK_MUTED,
    );
}

fn check_line(ui: &egui::Ui, pos: Pos2, done: bool, text: &str, s: f32) {
    let px = |v: f32| theme::px(v, s);
    let (glyph, color): (char, Color32) = if done {
        (theme::icon::CHECK, theme::SUCCESS)
    } else {
        (theme::icon::X, theme::INK_MUTED)
    };
    ui.painter().text(
        Pos2::new(pos.x + px(8.0), pos.y + px(8.0)),
        Align2::CENTER_CENTER,
        glyph.to_string(),
        theme::icon_font(px(15.0), false),
        color,
    );
    ui.painter().text(
        Pos2::new(pos.x + px(24.0), pos.y + px(8.0)),
        Align2::LEFT_CENTER,
        text,
        FontId::new(px(13.0), theme::family_medium()),
        if done {
            theme::INK_DIM
        } else {
            theme::INK_MUTED
        },
    );
}
