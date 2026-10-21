#![allow(dead_code)]

use eframe::egui::Key;

use crate::app::Page;
use crate::ui::theme::icon;

pub struct NavItem {
    pub page: Page,
    pub glyph: char,
    pub label: &'static str,
    pub shortcut: Key,
    pub sub_tabs: &'static [&'static str],
}

pub const ITEMS: [NavItem; 6] = [
    NavItem {
        page: Page::Home,
        glyph: icon::HOUSE,
        label: "Home",
        shortcut: Key::Num1,
        sub_tabs: &[],
    },
    NavItem {
        page: Page::Projects,
        glyph: icon::STACK,
        label: "Projects",
        shortcut: Key::Num2,
        sub_tabs: &[],
    },
    NavItem {
        page: Page::Build,
        glyph: icon::ROCKET,
        label: "Build",
        shortcut: Key::Num3,
        sub_tabs: &[],
    },
    NavItem {
        page: Page::History,
        glyph: icon::HISTORY,
        label: "History",
        shortcut: Key::Num4,
        sub_tabs: &["Builds", "Backups", "Patch notes", "Analytics"],
    },
    NavItem {
        page: Page::Pipelines,
        glyph: icon::FLOW,
        label: "Pipelines",
        shortcut: Key::Num5,
        sub_tabs: &["Pipelines", "Queue", "Schedule"],
    },
    NavItem {
        page: Page::Settings,
        glyph: icon::GEAR,
        label: "Settings",
        shortcut: Key::Num6,
        sub_tabs: &["General", "Accounts", "Notifications", "Doctor", "Advanced"],
    },
];

pub fn index(page: Page) -> usize {
    ITEMS.iter().position(|i| i.page == page).unwrap_or(0)
}

pub fn item(page: Page) -> &'static NavItem {
    &ITEMS[index(page)]
}
