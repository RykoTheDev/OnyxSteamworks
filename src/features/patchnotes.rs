use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::Project;

mod llm;
pub use llm::generate_llm;

#[derive(Debug, Clone, PartialEq)]
pub struct Commit {
    pub hash: String,
    pub subject: String,
}

const GROUPS: [(&str, &str); 7] = [
    ("feat", "Features"),
    ("fix", "Fixes"),
    ("perf", "Performance"),
    ("balance", "Balance"),
    ("art", "Art"),
    ("audio", "Audio"),
    ("other", "Other"),
];

pub fn repo_path(project: &Project) -> PathBuf {
    if project.repo_path.as_os_str().is_empty() {
        crate::features::maintenance::resolve(project).content_root
    } else {
        project.repo_path.clone()
    }
}

fn git(repo: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo);
    for arg in args {
        cmd.arg(arg);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        _ => None,
    }
}

pub fn head_commit(project: &Project) -> Option<String> {
    let repo = repo_path(project);
    if repo.as_os_str().is_empty() {
        return None;
    }
    git(&repo, &["rev-parse", "HEAD"]).filter(|s| !s.is_empty())
}

pub fn commits_since(project: &Project, since: Option<&str>) -> Vec<Commit> {
    let repo = repo_path(project);
    let range = match since {
        Some(c) if !c.is_empty() => format!("{c}..HEAD"),
        _ => "HEAD".to_string(),
    };
    let out = git(
        &repo,
        &[
            "log",
            "--no-merges",
            "--pretty=format:%h|%s",
            "-n",
            "200",
            &range,
        ],
    )
    .unwrap_or_default();
    parse_commits(&out)
}

fn parse_commits(out: &str) -> Vec<Commit> {
    out.lines()
        .filter_map(|line| {
            let (hash, subject) = line.split_once('|')?;
            if hash.trim().is_empty() {
                return None;
            }
            Some(Commit {
                hash: hash.trim().to_string(),
                subject: subject.trim().to_string(),
            })
        })
        .collect()
}

fn classify(subject: &str) -> (&'static str, String) {
    if let Some(colon) = subject.find(':') {
        let head = &subject[..colon];
        let typ = head
            .split(['(', '!'])
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        for (key, _) in GROUPS.iter().take(6) {
            if typ == *key {
                return (key, subject[colon + 1..].trim().to_string());
            }
        }
    }
    ("other", subject.trim().to_string())
}

pub fn group_markdown(commits: &[Commit]) -> String {
    if commits.is_empty() {
        return "No new commits since the last successful build.".to_string();
    }
    let mut buckets: Vec<Vec<String>> = vec![Vec::new(); GROUPS.len()];
    for commit in commits {
        let (key, message) = classify(&commit.subject);
        let idx = GROUPS
            .iter()
            .position(|(k, _)| *k == key)
            .unwrap_or(GROUPS.len() - 1);
        if !message.is_empty() {
            buckets[idx].push(message);
        }
    }
    let mut out = String::new();
    for (i, (_, label)) in GROUPS.iter().enumerate() {
        if buckets[i].is_empty() {
            continue;
        }
        out.push_str(&format!("## {label}\n"));
        for message in &buckets[i] {
            out.push_str(&format!("- {message}\n"));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

pub fn to_plain(markdown: &str) -> String {
    markdown
        .lines()
        .map(|line| {
            if let Some(h) = line.strip_prefix("## ") {
                format!("{h}:")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn to_bbcode(markdown: &str) -> String {
    let mut out = String::new();
    let mut in_list = false;
    for line in markdown.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            if in_list {
                out.push_str("[/list]\n");
                in_list = false;
            }
            out.push_str(&format!("[b]{h}[/b]\n"));
        } else if let Some(b) = line.strip_prefix("- ") {
            if !in_list {
                out.push_str("[list]\n");
                in_list = true;
            }
            out.push_str(&format!("[*]{b}\n"));
        } else {
            if in_list {
                out.push_str("[/list]\n");
                in_list = false;
            }
            out.push_str(line);
            out.push('\n');
        }
    }
    if in_list {
        out.push_str("[/list]");
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(subject: &str) -> Commit {
        Commit {
            hash: "abc123".into(),
            subject: subject.into(),
        }
    }

    #[test]
    fn parses_hash_subject_lines() {
        let got = parse_commits("abc|feat: new thing\ndef|fix: a bug\n|skip");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].hash, "abc");
        assert_eq!(got[1].subject, "fix: a bug");
    }

    #[test]
    fn classifies_conventional_prefixes() {
        assert_eq!(
            classify("feat(ui): add panel"),
            ("feat", "add panel".to_string())
        );
        assert_eq!(classify("fix!: crash"), ("fix", "crash".to_string()));
        assert_eq!(
            classify("balance: nerf boss"),
            ("balance", "nerf boss".to_string())
        );
        assert_eq!(
            classify("random message"),
            ("other", "random message".to_string())
        );
        assert_eq!(
            classify("chore: deps"),
            ("other", "chore: deps".to_string())
        );
    }

    #[test]
    fn groups_into_markdown_sections() {
        let md = group_markdown(&[commit("feat: a"), commit("fix: b"), commit("noise")]);
        assert!(md.contains("## Features"));
        assert!(md.contains("- a"));
        assert!(md.contains("## Fixes"));
        assert!(md.contains("## Other"));
        assert!(md.find("## Features").unwrap() < md.find("## Fixes").unwrap());
    }

    #[test]
    fn converts_to_bbcode_and_plain() {
        let md = "## Features\n- one\n- two";
        let bb = to_bbcode(md);
        assert!(bb.contains("[b]Features[/b]"));
        assert!(bb.contains("[list]"));
        assert!(bb.contains("[*]one"));
        assert!(bb.contains("[/list]"));
        assert!(to_plain(md).contains("Features:"));
    }
}
