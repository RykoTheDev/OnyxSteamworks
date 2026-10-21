use std::io::Read;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn run_capture(mut command: Command, timeout_secs: u64) -> Option<(i32, String)> {
    command.stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = command.spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_string(&mut out);
                }
                return Some((status.code().unwrap_or(-1), out));
            }
            Ok(None) => {
                if start.elapsed().as_secs() >= timeout_secs {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(80));
            }
            Err(_) => return None,
        }
    }
}

pub fn can_connect(host: &str, port: u16, timeout_secs: u64) -> bool {
    let target = format!("{host}:{port}");
    let Ok(mut addrs) = target.to_socket_addrs() else {
        return false;
    };
    addrs.any(|addr| TcpStream::connect_timeout(&addr, Duration::from_secs(timeout_secs)).is_ok())
}

pub fn writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".onyx_doctor_write_test");
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

#[cfg(unix)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c.as_ptr(), &mut st) };
    if rc != 0 {
        return None;
    }
    Some(st.f_bavail as u64 * st.f_frsize as u64)
}

#[cfg(windows)]
pub fn free_space(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut free: u64 = 0;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 { None } else { Some(free) }
}

#[cfg(not(any(unix, windows)))]
pub fn free_space(_path: &Path) -> Option<u64> {
    None
}
