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
    pub(super) logged_in: bool,
}

pub(super) fn finish(parsed: &Arc<Mutex<Parsed>>, code: Option<i32>) -> BuildResult {
    let (app_id, failure, logged_in) = {
        let p = parsed.lock().unwrap_or_else(|e| e.into_inner());
        (p.app_id.clone(), p.failure.clone(), p.logged_in)
    };
    if let Some(app_id) = app_id {
        return BuildResult::Success { app_id };
    }
    if let Some(message) = failure {
        return BuildResult::Failure { message };
    }
    if logged_in && code == Some(0) {
        return BuildResult::Success {
            app_id: String::new(),
        };
    }
    let message = match code {
        Some(0) => "steamcmd exited without a success marker".to_string(),
        Some(c) => format!("steamcmd exited with code {c}"),
        None => "steamcmd exited abnormally".to_string(),
    };
    BuildResult::Failure { message }
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

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    while let Some(&n) = chars.peek() {
                        chars.next();
                        if ('@'..='~').contains(&n) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    while let Some(&n) = chars.peek() {
                        chars.next();
                        if n == '\u{07}' || n == '\u{1b}' {
                            break;
                        }
                    }
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
        } else if c.is_control() && c != '\t' {
        } else {
            out.push(c);
        }
    }
    out
}

fn handle_line(
    raw: &str,
    tx: &Sender<BuildEvent>,
    ctx: &egui::Context,
    log: &Arc<Mutex<std::fs::File>>,
    parsed: &Arc<Mutex<Parsed>>,
    stdout: bool,
) {
    let cleaned = strip_ansi(raw);
    let line = cleaned.trim_end();
    if line.is_empty() {
        return;
    }
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
    match patterns::detect_login(line) {
        Some(true) => {
            if let Ok(mut g) = parsed.lock() {
                g.logged_in = true;
            }
        }
        Some(false) => {
            let reason = patterns::login_failure_reason(line);
            if let Ok(mut g) = parsed.lock()
                && g.failure.is_none()
            {
                g.failure = Some(reason);
            }
        }
        None => {}
    }
    if let Some(msg) = patterns::detect_failure(line)
        && let Ok(mut g) = parsed.lock()
        && g.failure.is_none()
    {
        g.failure = Some(msg);
    }
    ctx.request_repaint();
}
