//! Private immutable approval records retained before invoking ICP.

#[cfg(test)]
mod tests;

use ic_host_fs::{Commit, Destination, Directory, Policy, Retention};
use std::{
    ffi::OsStr,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);

fn prepare(path: &Path) -> Result<Directory> {
    let names = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(name) => Some(Ok(name)),
            Component::RootDir | Component::CurDir => None,
            _ => Some(Err("approval path cannot contain parent traversal")),
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let anchor = if path.is_absolute() {
        Path::new("/")
    } else {
        Path::new(".")
    };
    let directory = Directory::open(anchor, 0)?.ok_or("approval root disappeared")?;
    Ok(directory.prepare_directories(&names, 0o700, 0)?)
}

pub(super) fn attempt(root: &Path) -> Result<(PathBuf, Directory)> {
    let directory = prepare(root)?;
    for _ in 0..32 {
        let name = format!(
            "attempt-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        match directory.create_directory(OsStr::new(&name), 0o700, 0) {
            Ok(held) => return Ok((root.join(name), held)),
            Err(ic_host_fs::Error::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(format!("approval directory {name} may be retained: {e}").into()),
        }
    }
    Err("approval directory collision budget exhausted".into())
}

pub(super) fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let raw = path.as_os_str().as_encoded_bytes();
    if matches!(
        raw.rsplit(|b| *b == b'/').next(),
        None | Some(b"" | b"." | b"..")
    ) {
        return Err("approval needs a normal filename".into());
    }
    let name = path.file_name().ok_or("approval needs a filename")?;
    let parent = prepare(path.parent().unwrap_or(Path::new(".")))?;
    write_in(&parent, name, bytes)
}

pub(super) fn write_in(parent: &Directory, name: &OsStr, bytes: &[u8]) -> Result<()> {
    let stage = format!(
        ".approval-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let sealed = parent
        .stage(
            OsStr::new(&stage),
            name,
            Policy {
                destination: Destination::CreateNew,
                permissions: 0o600,
                retention: Retention::Discard,
            },
        )?
        .write_from(
            &mut &*bytes,
            ic_host_artifacts::Length::Exactly(bytes.len() as u64),
        )?
        .seal()?;
    match sealed.commit() {
        Commit::Committed(_) => Ok(()),
        Commit::NotPublished {cause, stage} => Err(format!("approval not published: {cause}; cleanup: {:?}", stage.abort().err()).into()),
        Commit::PublishedUnconfirmed {receipt, phase, cause, sync_error} => Err(format!("approval visible but unconfirmed: {receipt:?}; {phase:?}; {cause}; sync: {sync_error:?}").into()),
    }
}
