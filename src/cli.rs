mod exec;

use clap::{Args, Parser, Subcommand};

use crate::model::Project;
use exec::{FAILURE, SUCCESS, VALIDATION};

#[derive(Parser)]
#[command(name = "onyx", about = "Headless build and upload for SteamPipe GUI projects")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Build(BuildArgs),
    ListProjects,
    Validate(ValidateArgs),
    Doctor,
}

#[derive(Args)]
pub struct BuildArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub desc: Option<String>,
    #[arg(long)]
    pub preview: bool,
    #[arg(long = "no-diff")]
    pub no_diff: bool,
}

#[derive(Args)]
pub struct ValidateArgs {
    #[arg(long)]
    pub project: String,
}

#[cfg(windows)]
fn attach_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

pub fn run() -> i32 {
    #[cfg(windows)]
    attach_console();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let code = if e.use_stderr() { VALIDATION } else { SUCCESS };
            let _ = e.print();
            return code;
        }
    };
    match cli.command {
        Command::Build(args) => build_cmd(args),
        Command::ListProjects => list_projects(),
        Command::Validate(args) => validate_cmd(&args.project),
        Command::Doctor => doctor(),
    }
}

fn find_project<'a>(projects: &'a [Project], reference: &str) -> Option<&'a Project> {
    projects
        .iter()
        .find(|p| p.id == reference)
        .or_else(|| projects.iter().find(|p| p.name.eq_ignore_ascii_case(reference)))
}

fn load_projects() -> Result<Vec<Project>, i32> {
    crate::config::list_projects().map_err(|e| {
        eprintln!("could not load projects: {e}");
        FAILURE
    })
}

fn build_cmd(args: BuildArgs) -> i32 {
    let projects = match load_projects() {
        Ok(p) => p,
        Err(code) => return code,
    };
    let Some(project) = find_project(&projects, &args.project) else {
        eprintln!("no project matching '{}'", args.project);
        return VALIDATION;
    };
    let branch = args.branch.unwrap_or_default();
    let desc = args.desc.unwrap_or_default();
    exec::execute_build(project, &branch, &desc, args.preview, args.no_diff)
}

fn list_projects() -> i32 {
    match load_projects() {
        Ok(projects) => {
            for p in &projects {
                let app = if p.app_id.is_empty() { "-" } else { p.app_id.as_str() };
                println!("{}\t{}\tapp {}", p.name, p.id, app);
            }
            SUCCESS
        }
        Err(code) => code,
    }
}

fn validate_cmd(reference: &str) -> i32 {
    let projects = match load_projects() {
        Ok(p) => p,
        Err(code) => return code,
    };
    let Some(project) = find_project(&projects, reference) else {
        eprintln!("no project matching '{reference}'");
        return VALIDATION;
    };
    let errors = crate::validate::validate_project(project);
    if errors.is_empty() {
        println!("{} is valid", project.name);
        SUCCESS
    } else {
        for e in &errors {
            eprintln!("{}: {}", e.field, e.message);
        }
        VALIDATION
    }
}

fn doctor() -> i32 {
    println!("Onyx doctor");
    match crate::config::data_dir() {
        Ok(dir) => println!("data directory: {}", dir.display()),
        Err(e) => println!("data directory: unavailable ({e})"),
    }
    let projects = crate::config::list_projects().unwrap_or_default();
    println!("projects: {}", projects.len());
    for p in &projects {
        let steam = if p.steamcmd_path.is_file() { "ok" } else { "missing" };
        let content = if p.content_root.is_dir() { "ok" } else { "missing" };
        println!("  {} — steamcmd: {steam}, content root: {content}", p.name);
    }
    SUCCESS
}

fn quote(value: &str) -> String {
    if value.is_empty() || value.contains(' ') {
        format!("\"{value}\"")
    } else {
        value.to_string()
    }
}

pub fn build_command_string(project: &Project, branch: &str, desc: &str, preview: bool, no_diff: bool) -> String {
    let reference = if project.name.is_empty() { project.id.as_str() } else { project.name.as_str() };
    let mut parts = vec!["onyx".to_string(), "build".to_string(), "--project".to_string(), quote(reference)];
    if !branch.is_empty() {
        parts.push("--branch".to_string());
        parts.push(quote(branch));
    }
    if !desc.trim().is_empty() {
        parts.push("--desc".to_string());
        parts.push(quote(desc));
    }
    if preview {
        parts.push("--preview".to_string());
    }
    if no_diff {
        parts.push("--no-diff".to_string());
    }
    parts.join(" ")
}

pub fn ci_workflow_yaml(project: &Project) -> String {
    let reference = if project.name.is_empty() { project.id.as_str() } else { project.name.as_str() };
    let branch = project
        .branches
        .first()
        .map(|b| b.name.as_str())
        .filter(|n| !n.is_empty())
        .unwrap_or("default");
    format!(
        "# GitHub Actions workflow generated by SteamPipe GUI.\n\
         #\n\
         # Steam login in CI: SteamCMD needs a valid, already-cached login. Interactive\n\
         # Steam Guard cannot run in CI, so authenticate once on a trusted machine, then\n\
         # store the cached session (the SteamCMD config directory, e.g. a base64-encoded\n\
         # config.vdf) as an encrypted repository secret and restore it before the build.\n\
         # This is a starting template only: verify the current approach — for example\n\
         # game-ci/steam-deploy — against the latest documentation before relying on it,\n\
         # as Steam's authentication flow changes over time.\n\
         name: Deploy to Steam\n\
         on:\n\
         \x20\x20workflow_dispatch:\n\
         jobs:\n\
         \x20\x20deploy:\n\
         \x20\x20\x20\x20runs-on: windows-latest\n\
         \x20\x20\x20\x20steps:\n\
         \x20\x20\x20\x20\x20\x20- uses: actions/checkout@v4\n\
         \x20\x20\x20\x20\x20\x20- name: Build Onyx\n\
         \x20\x20\x20\x20\x20\x20\x20\x20run: cargo build --release\n\
         \x20\x20\x20\x20\x20\x20- name: Upload to Steam\n\
         \x20\x20\x20\x20\x20\x20\x20\x20env:\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20STEAM_CONFIG_VDF: ${{{{ secrets.STEAM_CONFIG_VDF }}}}\n\
         \x20\x20\x20\x20\x20\x20\x20\x20run: |\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20# TODO: restore the cached Steam login from STEAM_CONFIG_VDF before this step.\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20./target/release/onyx build --project \"{reference}\" --branch \"{branch}\"\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_build_with_all_flags() {
        let cli = Cli::try_parse_from([
            "onyx", "build", "--project", "Game", "--branch", "beta", "--desc", "notes", "--preview", "--no-diff",
        ])
        .expect("parse");
        match cli.command {
            Command::Build(a) => {
                assert_eq!(a.project, "Game");
                assert_eq!(a.branch.as_deref(), Some("beta"));
                assert_eq!(a.desc.as_deref(), Some("notes"));
                assert!(a.preview);
                assert!(a.no_diff);
            }
            _ => panic!("expected build"),
        }
    }

    #[test]
    fn parses_other_subcommands() {
        assert!(matches!(Cli::try_parse_from(["onyx", "list-projects"]).unwrap().command, Command::ListProjects));
        assert!(matches!(Cli::try_parse_from(["onyx", "doctor"]).unwrap().command, Command::Doctor));
        assert!(matches!(Cli::try_parse_from(["onyx", "validate", "--project", "G"]).unwrap().command, Command::Validate(_)));
    }

    #[test]
    fn build_requires_a_project() {
        assert!(Cli::try_parse_from(["onyx", "build"]).is_err());
    }

    #[test]
    fn command_string_includes_set_flags_only() {
        let project = Project { name: "My Game".into(), ..Default::default() };
        let cmd = build_command_string(&project, "beta", "", true, false);
        assert_eq!(cmd, "onyx build --project \"My Game\" --branch beta --preview");
    }

    #[test]
    fn ci_yaml_warns_about_login_and_cites_reference() {
        let project = Project { name: "Game".into(), ..Default::default() };
        let yaml = ci_workflow_yaml(&project);
        assert!(yaml.contains("game-ci/steam-deploy"));
        assert!(yaml.contains("secrets.STEAM_CONFIG_VDF"));
        assert!(yaml.contains("onyx build --project \"Game\""));
    }
}
