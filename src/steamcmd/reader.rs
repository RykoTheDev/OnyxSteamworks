use std::io::{Read, Write};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use eframe::egui;

use super::{BuildEvent, BuildResult, PromptKind, Stage};
use crate::steamcmd_patterns as patterns;

#[derive(Default)]
pub(super) struct Parsed {
    pub(super) app_id: Option<String>,
    pub(super) failure: Option<String>,
}

pub(super) fn finish(
    parsed: &Arc<Mutex<Parsed>>,
    status: std::io::Result<std::process::ExitStatus>,
) -> BuildResult {
    let (app_id, failure) = {
        let p = parsed.lock().unwrap_or_else(|e| e.into_inner());
        (p.app_id.clone(), p.failure.clone())
    };
    match (app_id, failure) {
        (Some(app_id), _) => BuildResult::Success { app_id },
        (None, Some(message)) => BuildResult::Failure { message },
        (None, None) => {
            let code = status.ok().and_then(|s| s.code());
            let message = match code {
                Some(0) => "steamcmd exited without a success marker".to_string(),
                Some(c) => format!("steamcmd exited with code {c}"),
                None => "steamcmd exited abnormally".to_string(),
            };
            BuildResult::Failure { message }
        }
    }
}

pub(super) fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    tx: Sender<BuildEvent>,
    ctx: egui::Context,
    log: Arc<Mutex<std::fs::File>>,
    parsed: Arc<Mutex<Parsed>>,
    stdout: bool,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut last_prompt: Option<PromptKind> = None;
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    for &byte in &chunk[..n] {
                        if byte == b'\n' || byte == b'\r' {
                            if !buf.is_empty() {
                                let line = String::from_utf8_lossy(&buf).to_string();
                                buf.clear();
                                handle_line(&line, &tx, &ctx, &log, &parsed, stdout);
                            }
                            last_prompt = None;
                        } else {
                            buf.push(byte);
                        }
                    }
                    if !buf.is_empty() {
                        let partial = String::from_utf8_lossy(&buf);
                        if let Some(kind) = patterns::detect_prompt(&partial)
                            && last_prompt != Some(kind)
                        {
                            last_prompt = Some(kind);
                            let _ = tx.send(BuildEvent::Prompt(kind));
                            ctx.request_repaint();
                        }
                    }
                }
                Err(_) => break,
            }
        }
        if !buf.is_empty() {
            let line = String::from_utf8_lossy(&buf).to_string();
            handle_line(&line, &tx, &ctx, &log, &parsed, stdout);
        }
    })
}

fn handle_line(
    line: &str,
    tx: &Sender<BuildEvent>,
    ctx: &egui::Context,
    log: &Arc<Mutex<std::fs::File>>,
    parsed: &Arc<Mutex<Parsed>>,
    stdout: bool,
) {
    if let Ok(mut f) = log.lock() {
        let _ = writeln!(f, "{line}");
    }
    let _ = tx.send(BuildEvent::Log(line.to_string()));
    if stdout {
        if let Some(stage) = patterns::detect_stage(line) {
            let _ = tx.send(BuildEvent::Stage(stage));
        }
        if let Some(p) = patterns::detect_progress(line) {
            let _ = tx.send(BuildEvent::Progress(p));
        }
        if let Some(app_id) = patterns::detect_success(line) {
            if let Ok(mut g) = parsed.lock() {
                g.app_id = Some(app_id);
            }
            let _ = tx.send(BuildEvent::Stage(Stage::Finished));
        }
    }
    if let Some(false) = patterns::detect_login(line)
        && let Ok(mut g) = parsed.lock()
        && g.failure.is_none()
    {
        g.failure = Some(line.trim().to_string());
    }
    if let Some(msg) = patterns::detect_failure(line)
        && let Ok(mut g) = parsed.lock()
        && g.failure.is_none()
    {
        g.failure = Some(msg);
    }
    ctx.request_repaint();
}
