use crate::app::Page;
use crate::model::Project;
use crate::ui::nav;

#[derive(Clone)]
pub enum Command {
    Goto(Page, usize),
    SwitchProject(usize),
    StartBuild,
    PreviewBuild,
    CancelBuild,
    OpenSteamworks,
    RunDoctor,
    ExportCi,
}

pub struct Action {
    pub id: String,
    pub title: String,
    pub keywords: String,
    pub shortcut: Option<String>,
    pub command: Command,
}

impl Action {
    fn new(
        id: &str,
        title: &str,
        keywords: &str,
        shortcut: Option<&str>,
        command: Command,
    ) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            keywords: keywords.to_string(),
            shortcut: shortcut.map(|s| s.to_string()),
            command,
        }
    }

    pub fn haystack(&self) -> String {
        format!("{} {}", self.title, self.keywords)
    }
}

pub fn registry(projects: &[Project]) -> Vec<Action> {
    let mut out = Vec::new();

    for (i, item) in nav::ITEMS.iter().enumerate() {
        let sc = format!("Ctrl+{}", i + 1);
        out.push(Action::new(
            &format!("goto.{}", item.label.to_ascii_lowercase()),
            &format!("Go to {}", item.label),
            "navigate page open",
            Some(&sc),
            Command::Goto(item.page, 0),
        ));
        for (ti, tab) in item.sub_tabs.iter().enumerate() {
            out.push(Action::new(
                &format!("goto.{}.{ti}", item.label.to_ascii_lowercase()),
                &format!("{} \u{203a} {}", item.label, tab),
                "navigate tab open",
                None,
                Command::Goto(item.page, ti),
            ));
        }
    }

    out.push(Action::new(
        "build.start",
        "Start build",
        "run upload deploy",
        None,
        Command::StartBuild,
    ));
    out.push(Action::new(
        "build.preview",
        "Run preview build",
        "dry run test",
        None,
        Command::PreviewBuild,
    ));
    out.push(Action::new(
        "build.cancel",
        "Cancel build",
        "stop abort",
        None,
        Command::CancelBuild,
    ));
    out.push(Action::new(
        "open.steamworks",
        "Open Steamworks page",
        "partner builds web",
        None,
        Command::OpenSteamworks,
    ));
    out.push(Action::new(
        "doctor.run",
        "Run Doctor",
        "diagnostics checks health",
        None,
        Command::RunDoctor,
    ));
    out.push(Action::new(
        "ci.export",
        "Export CI workflow",
        "github actions yaml",
        None,
        Command::ExportCi,
    ));

    for (i, p) in projects.iter().enumerate() {
        let name = if p.name.is_empty() {
            "Untitled".to_string()
        } else {
            p.name.clone()
        };
        out.push(Action::new(
            &format!("project.{i}"),
            &format!("Switch to {name}"),
            "project select active change",
            None,
            Command::SwitchProject(i),
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_includes_core_and_project_actions() {
        let projects = vec![Project {
            name: "Alpha".into(),
            ..Default::default()
        }];
        let actions = registry(&projects);
        assert!(actions.iter().any(|a| a.id == "goto.home"));
        assert!(
            actions
                .iter()
                .any(|a| a.title == "History \u{203a} Analytics")
        );
        assert!(actions.iter().any(|a| a.id == "build.start"));
        assert!(actions.iter().any(|a| a.title == "Switch to Alpha"));
    }

    #[test]
    fn nav_actions_carry_ctrl_shortcuts() {
        let actions = registry(&[]);
        let home = actions.iter().find(|a| a.id == "goto.home").expect("home");
        assert_eq!(home.shortcut.as_deref(), Some("Ctrl+1"));
    }
}
