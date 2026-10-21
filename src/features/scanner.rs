use std::collections::HashSet;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::WalkDir;

use crate::model::Project;
use crate::validate::{Fix, Issue, human_size, preflight_checks};

const MAX_TEXT: u64 = 2 * 1024 * 1024;
const OVERSIZE: u64 = 2 * 1024 * 1024 * 1024;

const SECRET_GLOBS: [&str; 8] = [
    ".env",
    "id_rsa",
    ".pem",
    ".pfx",
    ".key",
    "credentials.json",
    "secrets.*",
    ".keystore",
];
const JUNK_GLOBS: [&str; 7] = [
    "steam_appid.txt",
    ".pdb",
    ".ilk",
    ".exp",
    ".log",
    "Thumbs.db",
    ".DS_Store",
];
const JUNK_DIRS: [&str; 4] = [".git", ".vs", ".idea", "__MACOSX"];

fn normalize(pattern: &str) -> String {
    if pattern.starts_with('.') && !pattern.contains('*') && !pattern.contains('/') {
        format!("*{pattern}")
    } else {
        pattern.to_string()
    }
}

fn build_set(patterns: &[&str]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        if let Ok(glob) = Glob::new(&normalize(pat)) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

fn build_exclusions(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        let pat = pat.trim();
        if pat.is_empty() {
            continue;
        }
        if let Ok(glob) = Glob::new(&normalize(pat)) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

pub fn is_private_key(line: &str) -> bool {
    line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----")
}

pub fn has_aws_key(line: &str) -> bool {
    let b = line.as_bytes();
    b.windows(4).enumerate().any(|(i, w)| {
        w == b"AKIA"
            && b.len() >= i + 20
            && b[i + 4..i + 20]
                .iter()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    })
}

pub fn has_discord_webhook(line: &str) -> bool {
    line.contains("discord.com/api/webhooks/") || line.contains("discordapp.com/api/webhooks/")
}

pub fn has_secret_assignment(line: &str) -> bool {
    const KEYS: [&str; 6] = [
        "api_key", "apikey", "api-key", "secret", "token", "password",
    ];
    let lower = line.to_ascii_lowercase();
    let Some(after_key) = KEYS
        .iter()
        .filter_map(|k| lower.find(k).map(|p| p + k.len()))
        .min()
    else {
        return false;
    };
    let rest = &line[after_key.min(line.len())..];
    let Some(sep) = rest.find(['=', ':']) else {
        return false;
    };
    let value = rest[sep + 1..].trim().trim_matches(['"', '\'']);
    value.split_whitespace().next().unwrap_or("").len() >= 20
}

fn scan_text(path: &Path, rel: &str, issues: &mut Vec<Issue>) {
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let head = &bytes[..bytes.len().min(8192)];
    if head.contains(&0u8) {
        return;
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return;
    };
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if is_private_key(line) {
            issues.push(Issue::error(format!("{rel}:{n} — private key")));
        }
        if has_aws_key(line) {
            issues.push(Issue::error(format!("{rel}:{n} — AWS access key")));
        }
        if has_discord_webhook(line) {
            issues.push(Issue::error(format!("{rel}:{n} — Discord webhook URL")));
        }
        if has_secret_assignment(line) {
            issues.push(Issue::error(format!("{rel}:{n} — hardcoded secret")));
        }
    }
}

fn in_junk_dir(path: &Path) -> bool {
    path.components()
        .any(|c| JUNK_DIRS.contains(&c.as_os_str().to_string_lossy().as_ref()))
}

pub fn scan(project: &Project) -> Vec<Issue> {
    let secret_set = build_set(&SECRET_GLOBS);
    let junk_set = build_set(&JUNK_GLOBS);
    let mut issues = Vec::new();

    for depot in &project.depots {
        if depot.local_path.as_os_str().is_empty() {
            continue;
        }
        let mut patterns = depot.exclusions.clone();
        patterns.extend(project.ignore_patterns.iter().cloned());
        let excluded = build_exclusions(&patterns);
        let depth = if depot.recursive { usize::MAX } else { 1 };

        for entry in WalkDir::new(&depot.local_path)
            .max_depth(depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if excluded.is_match(name.as_str()) {
                continue;
            }
            let rel = path
                .strip_prefix(&depot.local_path)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            if in_junk_dir(path) {
                issues.push(
                    Issue::warning(format!("{rel} — build or editor junk"))
                        .with_fix(Fix::AddExclusion(name.clone())),
                );
                continue;
            }
            if secret_set.is_match(name.as_str()) {
                issues.push(Issue::error(format!("{rel} — credential or key file")));
            } else if junk_set.is_match(name.as_str()) {
                issues.push(
                    Issue::warning(format!("{rel} — build or editor junk"))
                        .with_fix(Fix::AddExclusion(name.clone())),
                );
            }

            let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if len > OVERSIZE {
                issues.push(Issue::warning(format!(
                    "{rel} — large file ({})",
                    human_size(len)
                )));
            }
            if len <= MAX_TEXT {
                scan_text(path, &rel, &mut issues);
            }
        }
    }

    issues
}

pub fn preflight(project: &Project) -> Vec<Issue> {
    let mut issues = preflight_checks(project);
    issues.extend(scan(project));
    let ignored: HashSet<&String> = project.ignored_findings.iter().collect();
    issues.retain(|i| !ignored.contains(&i.message));
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Depot, Project};
    use crate::validate::Severity;
    use std::io::Write;

    #[test]
    fn detects_private_key_header() {
        assert!(is_private_key("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(!is_private_key("-----BEGIN CERTIFICATE-----"));
    }

    #[test]
    fn detects_aws_access_key() {
        assert!(has_aws_key("key = AKIAIOSFODNN7EXAMPLE"));
        assert!(!has_aws_key("key = AKIAshort"));
        assert!(!has_aws_key("no key here"));
    }

    #[test]
    fn detects_discord_webhook() {
        assert!(has_discord_webhook(
            "https://discord.com/api/webhooks/123/abc"
        ));
        assert!(has_discord_webhook(
            "https://discordapp.com/api/webhooks/1/x"
        ));
        assert!(!has_discord_webhook("https://example.com/webhooks/1"));
    }

    #[test]
    fn detects_secret_assignment_with_long_value() {
        assert!(has_secret_assignment(
            "api_key = \"abcdefghijklmnopqrstuvwxyz\""
        ));
        assert!(has_secret_assignment(
            "password: superlongsecretvalue123456"
        ));
        assert!(!has_secret_assignment("password: short"));
        assert!(!has_secret_assignment("the token is somewhere"));
    }

    fn write(dir: &Path, name: &str, body: &[u8]) {
        let mut f = std::fs::File::create(dir.join(name)).expect("create");
        f.write_all(body).expect("write");
    }

    fn depot_project(dir: &Path) -> Project {
        Project {
            depots: vec![Depot {
                depot_id: "1001".into(),
                local_path: dir.to_path_buf(),
                recursive: true,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn scan_flags_credential_file_as_error() {
        let dir = tempdir();
        write(&dir, ".env", b"SECRET=1\n");
        let issues = scan(&depot_project(&dir));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains(".env"))
        );
    }

    #[test]
    fn scan_flags_junk_with_exclusion_fix() {
        let dir = tempdir();
        write(&dir, "Thumbs.db", b"junk");
        let issues = scan(&depot_project(&dir));
        let junk = issues
            .iter()
            .find(|i| i.message.contains("Thumbs.db"))
            .expect("junk issue");
        assert_eq!(junk.severity, Severity::Warning);
        assert!(matches!(junk.fix, Some(Fix::AddExclusion(_))));
    }

    #[test]
    fn scan_flags_content_secret_without_showing_value() {
        let dir = tempdir();
        write(&dir, "config.txt", b"aws = AKIAIOSFODNN7EXAMPLE\n");
        let issues = scan(&depot_project(&dir));
        let hit = issues
            .iter()
            .find(|i| i.message.contains("AWS access key"))
            .expect("aws issue");
        assert!(!hit.message.contains("AKIA"));
    }

    #[test]
    fn preflight_filters_ignored_findings() {
        let dir = tempdir();
        write(&dir, "Thumbs.db", b"junk");
        let mut project = depot_project(&dir);
        let msg = scan(&project)
            .into_iter()
            .find(|i| i.message.contains("Thumbs.db"))
            .map(|i| i.message)
            .expect("junk message");
        project.ignored_findings.push(msg.clone());
        assert!(!preflight(&project).iter().any(|i| i.message == msg));
    }

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("onyx_scan_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).expect("mkdir");
        base
    }
}
