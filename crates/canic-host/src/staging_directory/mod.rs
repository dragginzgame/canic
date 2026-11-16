//! Canic staging names and collision budget; custody belongs to FS.
use crate::{Result, supplier};
use ic_host_fs::Directory;
use std::ffi::OsString;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn allocate(parent: &Directory, prefix: &str) -> Result<(OsString, Directory)> {
    allocate_with(parent, prefix, &SEQUENCE)
}

fn allocate_with(
    parent: &Directory,
    prefix: &str,
    sequence: &AtomicU64,
) -> Result<(OsString, Directory)> {
    // Both callers supply fixed, private prefixes. Only occupied candidates retry;
    // sync/admission failures retain their candidate and stop allocation.
    for _ in 0..32 {
        let name = OsString::from(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            sequence.fetch_add(1, Ordering::Relaxed)
        ));
        match parent.create_directory(&name, 0o700, 0) {
            Ok(held) => return Ok((name, held)),
            Err(ic_host_fs::Error::Io(cause)) if cause.kind() == io::ErrorKind::AlreadyExists => {}
            Err(cause) => {
                return Err(supplier(AllocationFailure { name, cause }));
            }
        }
    }
    Err(supplier(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Canic staging collision budget exhausted",
    )))
}

#[derive(Debug, thiserror::Error)]
#[error("staging allocation failed at {name:?}; any created directory is retained: {cause}")]
struct AllocationFailure {
    name: OsString,
    #[source]
    cause: ic_host_fs::Error,
}

#[cfg(test)]
mod tests;
