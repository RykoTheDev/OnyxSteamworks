use std::path::{Path, PathBuf};

use crate::model::Project;

pub fn config_vdf(steamcmd_path: &Path) -> Option<std::path::PathBuf> {
    steamcmd_path
        .parent()
        .map(|d| d.join("config").join("config.vdf"))
}

pub fn resolve_steamcmd(settings_path: &str, projects: &[Project]) -> Option<PathBuf> {
    let path = settings_path.trim();
    if !path.is_empty() {
        return Some(PathBuf::from(path));
    }
    projects
        .iter()
        .map(|p| p.steamcmd_path.clone())
        .find(|p| !p.as_os_str().is_empty())
}

pub fn is_authorized(steamcmd_path: &Path, account: &str) -> bool {
    let account = account.trim();
    if account.is_empty() {
        return false;
    }
    let Some(path) = config_vdf(steamcmd_path) else {
        return false;
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => text
            .to_ascii_lowercase()
            .contains(&account.to_ascii_lowercase()),
        Err(_) => false,
    }
}

pub fn cached_login(steamcmd_path: &Path, account: &str) -> Option<()> {
    is_authorized(steamcmd_path, account).then_some(())
}

pub fn cached_login_for_project(project: &Project) -> Option<()> {
    let exe = crate::features::maintenance::effective_steamcmd(project);
    cached_login(&exe, project.steam_username.trim())
}

pub fn add(accounts: &mut Vec<String>, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || accounts.iter().any(|a| a.eq_ignore_ascii_case(name)) {
        return false;
    }
    accounts.push(name.to_string());
    true
}

pub fn remove(accounts: &mut Vec<String>, name: &str) {
    accounts.retain(|a| !a.eq_ignore_ascii_case(name));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_dedupes_case_insensitively() {
        let mut accounts = Vec::new();
        assert!(add(&mut accounts, "Builder"));
        assert!(!add(&mut accounts, "builder"));
        assert!(!add(&mut accounts, "  "));
        assert_eq!(accounts, vec!["Builder"]);
        remove(&mut accounts, "BUILDER");
        assert!(accounts.is_empty());
    }

    #[test]
    fn resolve_prefers_override_then_project() {
        assert_eq!(
            resolve_steamcmd("D:/scmd/steamcmd.exe", &[]),
            Some(PathBuf::from("D:/scmd/steamcmd.exe"))
        );
        let projects = vec![
            Project::default(),
            Project {
                steamcmd_path: PathBuf::from("C:/a/steamcmd.exe"),
                ..Default::default()
            },
        ];
        assert_eq!(
            resolve_steamcmd("", &projects),
            Some(PathBuf::from("C:/a/steamcmd.exe"))
        );
        assert_eq!(resolve_steamcmd("  ", &[Project::default()]), None);
    }

    #[test]
    fn authorized_when_config_mentions_account() {
        let dir = std::env::temp_dir().join(format!("onyx_acct_{}", uuid::Uuid::new_v4()));
        let cfg = dir.join("config");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::write(cfg.join("config.vdf"), "\"Accounts\" { \"builder\" { } }").unwrap();
        let steamcmd = dir.join("steamcmd.exe");
        assert!(is_authorized(&steamcmd, "Builder"));
        assert!(!is_authorized(&steamcmd, "someone_else"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
