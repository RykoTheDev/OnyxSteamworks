use crate::model::Project;

pub fn steamworks_builds_url(app_id: &str) -> String {
    format!("https://partner.steamgames.com/apps/builds/{app_id}")
}

pub fn steam_run_url(app_id: &str) -> String {
    format!("steam://run/{app_id}")
}

fn quote(value: &str) -> String {
    if value.is_empty() || value.contains(' ') {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}

pub fn steamcmd_command(project: &Project) -> String {
    let project = &crate::features::maintenance::resolve(project);
    let exe = project.steamcmd_path.to_string_lossy();
    let mut parts = vec![
        quote(&exe),
        "+@ShutdownOnFailedCommand".to_string(),
        "1".to_string(),
        "+@NoPromptForPassword".to_string(),
        "1".to_string(),
    ];
    let user = project.steam_username.trim();
    if !user.is_empty() {
        parts.push("+login".to_string());
        parts.push(user.to_string());
    }
    parts.push("+run_app_build".to_string());
    parts.push(quote("<app_build.vdf>"));
    parts.push("+quit".to_string());
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_steamworks_and_run_urls() {
        assert_eq!(
            steamworks_builds_url("480"),
            "https://partner.steamgames.com/apps/builds/480"
        );
        assert_eq!(steam_run_url("480"), "steam://run/480");
    }

    #[test]
    fn steamcmd_command_includes_login_and_runbuild() {
        let project = Project {
            steamcmd_path: "C:/steamcmd.exe".into(),
            steam_username: "tester".into(),
            ..Default::default()
        };
        let cmd = steamcmd_command(&project);
        assert!(cmd.contains("+login tester"));
        assert!(cmd.contains("+run_app_build"));
        assert!(cmd.ends_with("+quit"));
    }

    #[test]
    fn steamcmd_command_omits_login_when_no_user() {
        let project = Project {
            steamcmd_path: "steamcmd".into(),
            ..Default::default()
        };
        let cmd = steamcmd_command(&project);
        assert!(!cmd.contains("+login"));
    }
}
