//! Canic's sidecar path and permission policy; no implicit parent creation.
use ic_host_fs::{Directory, Error, FilePolicy, Lock};
use std::path::Path;

pub(crate) fn acquire(path: &Path) -> Result<Lock, Error> {
    if path.as_os_str().as_encoded_bytes().ends_with(b"/") {
        return Err(Error::Name);
    }
    let name = path.file_name().ok_or(Error::Name)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize()?;
    let held = Directory::open(&parent, 0)?
        .ok_or_else(|| Error::Io(std::io::ErrorKind::NotFound.into()))?;
    held.lock(
        name,
        0o600,
        FilePolicy {
            forbidden: 0,
            single_link: false,
        },
    )
}
