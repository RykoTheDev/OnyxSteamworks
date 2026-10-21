#![allow(dead_code)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::model::Project;

mod reader;
#[cfg(test)]
mod tests;

use reader::{Parsed, finish, spawn_reader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Connecting,
    LoggingIn,
    Scanning,
    Uploading,
    Committing,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Password,
    SteamGuard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildResult {
    Success { app_id: String },
    Failure { message: String },
    Cancelled,
}

pub enum BuildEvent {
    Log(String),
    Stage(Stage),
    Progress(f32),
    Prompt(PromptKind),
    Finished(BuildResult),
}

#[derive(Debug, thiserror::Error)]
pub enum SteamError {
    #[error("failed to launch steamcmd: {0}")]
    Spawn(#[source] std::io::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct RunOptions {
    pub app_build_vdf: PathBuf,
    pub log_path: Option<PathBuf>,
    pub interactive: bool,
}

pub struct BuildHandle {
    pid: u32,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    cancelled: Arc<AtomicBool>,
    pub log_path: PathBuf,
}

impl BuildHandle {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn send_input(&self, text: &str) {
        if let Ok(mut guard) = self.stdin.lock()
            && let Some(stdin) = guard.as_mut()
        {
            let _ = writeln!(stdin, "{text}");
            let _ = stdin.flush();
        }
    }
}

fn log_path_for(explicit: Option<&PathBuf>, prefix: &str) -> Result<PathBuf, SteamError> {
    match explicit {
        Some(p) => Ok(p.clone()),
        None => {
            let dir = crate::config::log_dir()
                .map_err(|e| SteamError::Io(std::io::Error::other(e.to_string())))?;
            Ok(dir.join(format!("{prefix}-{}.log", uuid::Uuid::new_v4())))
        }
    }
}

pub fn start_build(
    project: &Project,
    options: &RunOptions,
    ctx: &egui::Context,
    tx: Sender<BuildEvent>,
) -> Result<BuildHandle, SteamError> {
    let log_path = log_path_for(options.log_path.as_ref(), "build")?;
    let mut cmd = Command::new(&project.steamcmd_path);
    cmd.arg("+@ShutdownOnFailedCommand").arg("1");
    if !options.interactive {
        cmd.arg("+@NoPromptForPassword").arg("1");
    }
    let login = project.steam_username.trim();
    if !login.is_empty() {
        cmd.arg("+login").arg(login);
    }
    cmd.arg("+run_app_build")
        .arg(&options.app_build_vdf)
        .arg("+quit");
    run(cmd, log_path, ctx, tx)
}

pub fn start_login(
    project: &Project,
    ctx: &egui::Context,
    tx: Sender<BuildEvent>,
) -> Result<BuildHandle, SteamError> {
    let log_path = log_path_for(None, "login")?;
    let mut cmd = Command::new(&project.steamcmd_path);
    cmd.arg("+@ShutdownOnFailedCommand").arg("1");
    let login = project.steam_username.trim();
    if !login.is_empty() {
        cmd.arg("+login").arg(login);
    }
    cmd.arg("+quit");
    run(cmd, log_path, ctx, tx)
}

pub struct VerifyOptions {
    pub app_id: String,
    pub install_dir: PathBuf,
    pub branch: String,
    pub beta_password: String,
    pub log_path: Option<PathBuf>,
}

pub fn start_verify(
    project: &Project,
    options: &VerifyOptions,
    ctx: &egui::Context,
    tx: Sender<BuildEvent>,
) -> Result<BuildHandle, SteamError> {
    let log_path = log_path_for(options.log_path.as_ref(), "verify")?;
    let mut cmd = Command::new(&project.steamcmd_path);
    cmd.arg("+@ShutdownOnFailedCommand")
        .arg("1")
        .arg("+@NoPromptForPassword")
        .arg("1");
    cmd.arg("+force_install_dir").arg(&options.install_dir);
    let login = project.steam_username.trim();
    if !login.is_empty() {
        cmd.arg("+login").arg(login);
    }
    cmd.arg("+app_update").arg(&options.app_id);
    let branch = options.branch.trim();
    if !branch.is_empty() && branch != "default" {
        cmd.arg("-beta").arg(branch);
        if !options.beta_password.trim().is_empty() {
            cmd.arg("-betapassword").arg(options.beta_password.trim());
        }
    }
    cmd.arg("validate").arg("+quit");
    run(cmd, log_path, ctx, tx)
}

fn run(
    mut cmd: Command,
    log_path: PathBuf,
    ctx: &egui::Context,
    tx: Sender<BuildEvent>,
) -> Result<BuildHandle, SteamError> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let log = Arc::new(Mutex::new(std::fs::File::create(&log_path)?));

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(SteamError::Spawn)?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = child.stdin.take();

    let cancelled = Arc::new(AtomicBool::new(false));
    let parsed = Arc::new(Mutex::new(Parsed::default()));

    let mut readers = Vec::new();
    if let Some(out) = stdout {
        readers.push(spawn_reader(
            out,
            tx.clone(),
            ctx.clone(),
            log.clone(),
            parsed.clone(),
            true,
        ));
    }
    if let Some(err) = stderr {
        readers.push(spawn_reader(
            err,
            tx.clone(),
            ctx.clone(),
            log.clone(),
            parsed.clone(),
            false,
        ));
    }

    let waiter_cancelled = cancelled.clone();
    let waiter_ctx = ctx.clone();
    std::thread::spawn(move || {
        for r in readers {
            let _ = r.join();
        }
        let status = child.wait();
        let result = if waiter_cancelled.load(Ordering::SeqCst) {
            BuildResult::Cancelled
        } else {
            finish(&parsed, status)
        };
        let _ = tx.send(BuildEvent::Finished(result));
        waiter_ctx.request_repaint();
    });

    Ok(BuildHandle {
        pid,
        stdin: Arc::new(Mutex::new(stdin)),
        cancelled,
        log_path,
    })
}

pub fn cancel(handle: &BuildHandle) {
    handle.cancelled.store(true, Ordering::SeqCst);
    kill_tree(handle.pid);
}

#[cfg(windows)]
fn kill_tree(pid: u32) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(windows))]
fn kill_tree(pid: u32) {
    let _ = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
