#![allow(dead_code)]

use std::collections::HashSet;

use crate::model::Project;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field}: {message}")]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

fn is_numeric(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

pub fn validate_project(project: &Project) -> Vec<FieldError> {
    let mut errors = Vec::new();

    if project.name.trim().is_empty() {
        errors.push(FieldError::new("name", "is required"));
    }
    if !is_numeric(&project.app_id) {
        errors.push(FieldError::new("app_id", "must be a number"));
    }
    if project.content_root.as_os_str().is_empty() {
        errors.push(FieldError::new("content_root", "is required"));
    }
    if project.build_output_dir.as_os_str().is_empty() {
        errors.push(FieldError::new("build_output_dir", "is required"));
    }
    if project.steamcmd_path.as_os_str().is_empty() {
        errors.push(FieldError::new("steamcmd_path", "is required"));
    }

    let mut seen = HashSet::new();
    for (i, depot) in project.depots.iter().enumerate() {
        if !is_numeric(&depot.depot_id) {
            errors.push(FieldError::new(
                format!("depots[{i}].depot_id"),
                "must be a number",
            ));
        } else if !seen.insert(depot.depot_id.clone()) {
            errors.push(FieldError::new(
                format!("depots[{i}].depot_id"),
                format!("duplicate depot id {}", depot.depot_id),
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Depot;

    fn valid_project() -> Project {
        Project {
            name: "Game".to_string(),
            app_id: "480".to_string(),
            content_root: "C:/content".into(),
            build_output_dir: "C:/out".into(),
            steamcmd_path: "C:/steamcmd.exe".into(),
            ..Default::default()
        }
    }

    #[test]
    fn accepts_a_valid_project() {
        assert!(validate_project(&valid_project()).is_empty());
    }

    #[test]
    fn flags_bad_app_id_and_missing_paths() {
        let mut p = valid_project();
        p.app_id = "not-a-number".to_string();
        p.content_root = Default::default();
        let errors = validate_project(&p);
        assert!(errors.iter().any(|e| e.field == "app_id"));
        assert!(errors.iter().any(|e| e.field == "content_root"));
    }

    #[test]
    fn flags_non_numeric_and_duplicate_depot_ids() {
        let mut p = valid_project();
        p.depots = vec![
            Depot {
                depot_id: "481".into(),
                ..Default::default()
            },
            Depot {
                depot_id: "481".into(),
                ..Default::default()
            },
            Depot {
                depot_id: "abc".into(),
                ..Default::default()
            },
        ];
        let errors = validate_project(&p);
        assert!(errors.iter().any(|e| e.field == "depots[1].depot_id"));
        assert!(errors.iter().any(|e| e.field == "depots[2].depot_id"));
    }
}
