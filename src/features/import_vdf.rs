use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};

use crate::kv;
use crate::model::{Depot, Project};

pub fn import_appbuild(path: &Path) -> anyhow::Result<Project> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let root = kv::parse(&text).map_err(|e| anyhow!("{e}"))?;
    let app = kv::find(&root, "appbuild")
        .and_then(|v| v.as_block())
        .ok_or_else(|| anyhow!("no \"appbuild\" block found"))?;

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| s != "appbuild")
        .unwrap_or_else(|| "Imported project".to_string());

    let mut project = Project {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        app_id: kv::get_str(app, "appid").unwrap_or_default().to_string(),
        content_root: PathBuf::from(kv::get_str(app, "contentroot").unwrap_or_default()),
        build_output_dir: PathBuf::from(kv::get_str(app, "buildoutput").unwrap_or_default()),
        desc_template: kv::get_str(app, "desc").unwrap_or_default().to_string(),
        set_live_branch: kv::get_str(app, "setlive").unwrap_or_default().to_string(),
        ..Default::default()
    };

    if let Some(depots) = kv::find(app, "depots").and_then(|v| v.as_block()) {
        for (depot_id, value) in depots {
            if let Some(file) = value.as_str() {
                let depot_path = resolve(dir, file);
                match import_depot(&depot_path) {
                    Ok(depot) => project.depots.push(depot),
                    Err(_) => project.depots.push(Depot {
                        depot_id: depot_id.clone(),
                        depot_path: ".".to_string(),
                        recursive: true,
                        ..Default::default()
                    }),
                }
            }
        }
    }
    Ok(project)
}

fn resolve(dir: &Path, file: &str) -> PathBuf {
    let file = file.replace('\\', "/");
    let p = PathBuf::from(&file);
    if p.is_absolute() { p } else { dir.join(file) }
}

pub fn import_depot(path: &Path) -> anyhow::Result<Depot> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let root = kv::parse(&text).map_err(|e| anyhow!("{e}"))?;
    let cfg = kv::find(&root, "DepotBuildConfig")
        .and_then(|v| v.as_block())
        .ok_or_else(|| anyhow!("no \"DepotBuildConfig\" block"))?;

    let mut depot = Depot {
        depot_id: kv::get_str(cfg, "DepotID").unwrap_or_default().to_string(),
        depot_path: ".".to_string(),
        recursive: true,
        ..Default::default()
    };
    if let Some(mapping) = kv::find(cfg, "FileMapping").and_then(|v| v.as_block()) {
        depot.local_path = PathBuf::from(kv::get_str(mapping, "LocalPath").unwrap_or_default());
        if let Some(dp) = kv::get_str(mapping, "DepotPath") {
            depot.depot_path = dp.to_string();
        }
        if let Some(r) = kv::get_str(mapping, "recursive") {
            depot.recursive = r.trim() == "1";
        }
    }
    depot.exclusions = kv::collect(cfg, "FileExclusion")
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    Ok(depot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("onyx_imp_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn imports_appbuild_with_depot() {
        let dir = tmp();
        std::fs::write(
            dir.join("appbuild.vdf"),
            "\"appbuild\"\n{\n\t\"appid\" \"480\"\n\t\"desc\" \"Nightly\"\n\t\"contentroot\" \"C:\\\\game\"\n\t\"buildoutput\" \"C:\\\\out\"\n\t\"setlive\" \"beta\"\n\t\"depots\" {\n\t\t\"481\" \"depot_build_481.vdf\"\n\t}\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("depot_build_481.vdf"),
            "// a depot\n\"DepotBuildConfig\" {\n\t\"DepotID\" \"481\"\n\t\"FileMapping\" {\n\t\t\"LocalPath\" \"C:\\\\game\\\\win\"\n\t\t\"DepotPath\" \".\"\n\t\t\"recursive\" \"1\"\n\t}\n\t\"FileExclusion\" \"*.pdb\"\n}\n",
        )
        .unwrap();

        let project = import_appbuild(&dir.join("appbuild.vdf")).expect("import");
        assert_eq!(project.app_id, "480");
        assert_eq!(project.set_live_branch, "beta");
        assert_eq!(project.depots.len(), 1);
        assert_eq!(project.depots[0].depot_id, "481");
        assert!(project.depots[0].recursive);
        assert_eq!(project.depots[0].exclusions, vec!["*.pdb"]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn errors_without_appbuild_block() {
        let dir = tmp();
        let p = dir.join("x.vdf");
        std::fs::write(&p, "\"other\" { \"a\" \"1\" }").unwrap();
        assert!(import_appbuild(&p).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
