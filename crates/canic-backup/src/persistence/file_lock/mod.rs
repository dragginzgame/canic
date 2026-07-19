//! Module: persistence::file_lock
//!
//! Responsibility: open and exclusively lock one regular no-follow sidecar file.
//! Does not own: journal or command-lifetime semantics, paths, or error projection.

use std::{fs, io, path::Path};

#[cfg(unix)]
use rustix::fs::{CWD, FileType, FlockOperation, Mode, OFlags, flock, fstat, openat};

#[derive(Debug)]
pub(super) enum FileLockError {
    Locked,
    UnsafeEntry { kind: String },
    Io(io::Error),
}

pub(super) fn acquire(path: &Path) -> Result<fs::File, FileLockError> {
    #[cfg(unix)]
    {
        acquire_supported(path)
    }

    #[cfg(not(unix))]
    {
        Err(FileLockError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "file locking is unsupported on this host: {}",
                path.display()
            ),
        )))
    }
}

#[cfg(unix)]
pub(super) fn unlock(file: &fs::File) {
    let _ = flock(file, FlockOperation::Unlock);
}

#[cfg(unix)]
fn acquire_supported(path: &Path) -> Result<fs::File, FileLockError> {
    reject_existing_unsafe_entry(path)?;

    let fd = openat(
        CWD,
        path,
        OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(errno_to_io)
    .map_err(FileLockError::Io)?;
    let metadata = fstat(&fd).map_err(errno_to_io).map_err(FileLockError::Io)?;
    let kind = FileType::from_raw_mode(metadata.st_mode);
    if !kind.is_file() {
        return Err(FileLockError::UnsafeEntry {
            kind: format!("{kind:?}"),
        });
    }

    let file = fs::File::from(fd);
    match flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => {}
        Err(error) if error == rustix::io::Errno::WOULDBLOCK => {
            return Err(FileLockError::Locked);
        }
        Err(error) => return Err(FileLockError::Io(errno_to_io(error))),
    }
    Ok(file)
}

#[cfg(unix)]
fn reject_existing_unsafe_entry(path: &Path) -> Result<(), FileLockError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(metadata) => Err(FileLockError::UnsafeEntry {
            kind: if metadata.file_type().is_symlink() {
                "Symlink".to_string()
            } else if metadata.file_type().is_dir() {
                "Directory".to_string()
            } else {
                "Special".to_string()
            },
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FileLockError::Io(error)),
    }
}

#[cfg(unix)]
fn errno_to_io(error: rustix::io::Errno) -> io::Error {
    io::Error::from_raw_os_error(error.raw_os_error())
}
