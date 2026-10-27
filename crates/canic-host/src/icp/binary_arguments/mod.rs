//! Private invocation scratch shared by external Candid and owned Wire calls.

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
const MAX_ARGUMENT_FILE_ATTEMPTS: usize = 32;
static NEXT_ARGUMENT_FILE: AtomicU64 = AtomicU64::new(0);

/// Remove invocation scratch without changing the observed remote-call outcome.
pub fn cleanup_binary_argument_file(path: &Path) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != io::ErrorKind::NotFound
    {
        eprintln!(
            "warning: could not remove binary argument file {}: {error}",
            path.display()
        );
    }
}

/// Write private invocation scratch, closing it before the child opens it.
///
/// Completed writes provide process visibility without a durable disk flush.
/// Recovery recreates these arguments from caller-owned durable intent.
pub fn write_binary_argument_file(bytes: &[u8]) -> io::Result<PathBuf> {
    for _ in 0..MAX_ARGUMENT_FILE_ATTEMPTS {
        let sequence = NEXT_ARGUMENT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "canic-raw-arguments-{}-{sequence}.bin",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        match options.open(&path) {
            Ok(mut file) => {
                if let Err(source) = file.write_all(bytes) {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(source);
                }
                return Ok(path);
            }
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(source),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique binary argument file",
    ))
}
