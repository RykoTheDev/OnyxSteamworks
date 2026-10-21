use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use eframe::egui;

pub const STEAMCMD_URL: &str = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";

#[derive(Debug, thiserror::Error)]
pub enum WizardError {
    #[error("download failed: {0}")]
    Http(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("steamcmd was not found after extraction")]
    NotFound,
}

pub enum DownloadMsg {
    Progress { done: u64, total: u64 },
    Extracting,
    Done(PathBuf),
    Error(String),
}

pub fn executable_name() -> &'static str {
    if cfg!(windows) {
        "steamcmd.exe"
    } else {
        "steamcmd.sh"
    }
}

pub fn locate_executable(dir: &Path) -> Option<PathBuf> {
    let direct = dir.join(executable_name());
    if direct.exists() {
        return Some(direct);
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some(executable_name()) {
            return Some(path);
        }
    }
    None
}

pub fn install_steamcmd(dest: PathBuf, tx: Sender<DownloadMsg>, ctx: egui::Context) {
    std::thread::spawn(move || match run_install(&dest, &tx, &ctx) {
        Ok(exe) => {
            let _ = tx.send(DownloadMsg::Done(exe));
            ctx.request_repaint();
        }
        Err(e) => {
            let _ = tx.send(DownloadMsg::Error(e.to_string()));
            ctx.request_repaint();
        }
    });
}

fn run_install(
    dest: &Path,
    tx: &Sender<DownloadMsg>,
    ctx: &egui::Context,
) -> Result<PathBuf, WizardError> {
    std::fs::create_dir_all(dest)?;
    let resp = ureq::get(STEAMCMD_URL)
        .call()
        .map_err(|e| WizardError::Http(e.to_string()))?;
    let total = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let zip_path = dest.join("steamcmd_download.zip");
    {
        let mut reader = resp.into_reader();
        let mut file = File::create(&zip_path)?;
        let mut buf = vec![0u8; 64 * 1024];
        let mut done = 0u64;
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            done += n as u64;
            let _ = tx.send(DownloadMsg::Progress { done, total });
            ctx.request_repaint();
        }
    }

    let _ = tx.send(DownloadMsg::Extracting);
    ctx.request_repaint();
    let file = File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(dest)?;
    let _ = std::fs::remove_file(&zip_path);

    locate_executable(dest).ok_or(WizardError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_name_is_platform_specific() {
        if cfg!(windows) {
            assert_eq!(executable_name(), "steamcmd.exe");
        } else {
            assert_eq!(executable_name(), "steamcmd.sh");
        }
    }

    #[test]
    fn locate_finds_executable_at_root() {
        let dir = std::env::temp_dir().join(format!("onyx_wiz_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        File::create(dir.join(executable_name())).unwrap();
        assert_eq!(locate_executable(&dir), Some(dir.join(executable_name())));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn locate_returns_none_when_absent() {
        let dir = std::env::temp_dir().join(format!("onyx_wiz_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        File::create(dir.join("readme.txt")).unwrap();
        assert_eq!(locate_executable(&dir), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn url_points_at_valve_cdn() {
        assert!(STEAMCMD_URL.ends_with("steamcmd.zip"));
        assert!(STEAMCMD_URL.starts_with("https://"));
    }
}
