use anyhow::{Context, Result, bail};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::Path,
};

pub fn lock_pid_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .with_context(|| format!("open Span daemon PID file {}", path.display()))?;
    anyhow::ensure!(
        file.metadata()?.is_file(),
        "Span daemon PID path is not a regular file"
    );

    // The descriptor remains open for the daemon's lifetime. Never unlink the
    // lock file, since another process could otherwise lock a different inode.
    // SAFETY: The descriptor is valid while `file` is open.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::WouldBlock {
            let mut pid = String::new();
            if (&file).take(32).read_to_string(&mut pid).is_ok()
                && let Ok(pid) = pid.trim().parse::<u32>()
            {
                bail!(
                    "Span daemon is already running (PID {pid}, {})",
                    path.display()
                );
            }
            bail!(
                "Span daemon is already running or starting ({})",
                path.display()
            );
        }
        return Err(error).with_context(|| format!("lock Span daemon PID file {}", path.display()));
    }
    file.set_len(0)?;
    writeln!(file, "{}", std::process::id())?;
    Ok(file)
}
