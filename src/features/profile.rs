use anyhow::{Context, anyhow};

use crate::model::Project;

pub fn export_json(project: &Project) -> anyhow::Result<String> {
    let mut clean = project.clone();
    clean.steam_username = String::new();
    clean.default_account = String::new();
    serde_json::to_string_pretty(&clean).context("encoding project JSON")
}

pub fn import_json(text: &str) -> anyhow::Result<Project> {
    let mut project: Project =
        serde_json::from_str(text).map_err(|e| anyhow!("not a valid project profile: {e}"))?;
    project.id = uuid::Uuid::new_v4().to_string();
    project.steam_username = String::new();
    project.default_account = String::new();
    if project.name.trim().is_empty() {
        project.name = "Imported project".to_string();
    }
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Depot;

    fn sample() -> Project {
        Project {
            id: "orig".into(),
            name: "Game".into(),
            app_id: "480".into(),
            steam_username: "builder".into(),
            default_account: "builder".into(),
            depots: vec![Depot {
                depot_id: "481".into(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn export_strips_account_fields() {
        let json = export_json(&sample()).expect("export");
        assert!(!json.contains("builder"));
        assert!(json.contains("480"));
    }

    #[test]
    fn roundtrip_assigns_new_id_and_keeps_depots() {
        let json = export_json(&sample()).expect("export");
        let back = import_json(&json).expect("import");
        assert_ne!(back.id, "orig");
        assert_eq!(back.app_id, "480");
        assert_eq!(back.depots.len(), 1);
        assert!(back.steam_username.is_empty());
    }

    #[test]
    fn import_rejects_garbage() {
        assert!(import_json("not json").is_err());
    }
}
