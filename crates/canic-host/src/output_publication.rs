//! Canic output destination policy; filesystem custody belongs to IC Host FS.
use crate::{Error, Result, supplier};
use ic_host_fs::{Commit, Destination, Directory, Policy, Retention};
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};
use std::sync::atomic::{AtomicU64, Ordering};

static STAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, thiserror::Error)]
enum Failure {
    #[error("output not published: {cause}; staging cleanup: {cleanup:?}")]
    NotPublished {
        #[source]
        cause: ic_host_fs::Error,
        cleanup: Option<ic_host_fs::Error>,
    },
    #[error(
        "output visible but unconfirmed at {phase:?}: {cause}; receipt: {receipt:?}; sync: {sync_error:?}"
    )]
    Unconfirmed {
        #[source]
        cause: ic_host_fs::Error,
        phase: ic_host_fs::Phase,
        receipt: ic_host_fs::Receipt,
        sync_error: Option<std::io::Error>,
    },
}

pub(crate) fn publish(
    destination: &Path,
    bytes: &[u8],
    mode: Destination,
    parent_mode: u32,
) -> Result<()> {
    let raw = destination.as_os_str().as_bytes();
    if raw.contains(&0)
        || matches!(
            raw.rsplit(|byte| *byte == b'/').next(),
            None | Some(b"" | b"." | b"..")
        )
    {
        return Err(Error::Invalid("output needs a normal filename".into()));
    }
    let name = destination
        .file_name()
        .ok_or_else(|| Error::Invalid("output needs a filename".into()))?;
    let path = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = prepare(path, parent_mode)?;
    publish_in(&parent, name, bytes, mode)
}

pub(crate) fn prepare(path: &Path, mode: u32) -> Result<Directory> {
    // Application path policy remains outside the filesystem product.
    let mut names = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(name) => names.push(name),
            Component::RootDir | Component::CurDir => {}
            _ => {
                return Err(Error::Invalid(
                    "output cannot contain parent traversal".into(),
                ));
            }
        }
    }
    let anchor = if path.is_absolute() {
        Path::new("/")
    } else {
        Path::new(".")
    };
    let root = Directory::open(anchor, 0)
        .map_err(supplier)?
        .ok_or_else(|| Error::Invalid("output root disappeared".into()))?;
    root.prepare_directories(&names, mode, 0).map_err(supplier)
}

pub(crate) fn publish_in(
    parent: &Directory,
    name: &OsStr,
    bytes: &[u8],
    mode: Destination,
) -> Result<()> {
    let staging = OsString::from(format!(
        ".canic-artifact-{}-{}",
        std::process::id(),
        STAGE.fetch_add(1, Ordering::Relaxed)
    ));
    let sealed = parent
        .stage(
            &staging,
            name,
            Policy {
                destination: mode,
                permissions: 0o600,
                retention: Retention::Discard,
            },
        )
        .map_err(supplier)?
        .write_from(
            &mut &*bytes,
            ic_host_artifacts::Length::Exactly(bytes.len() as u64),
        )
        .map_err(supplier)?
        .seal()
        .map_err(supplier)?;
    match sealed.commit() {
        Commit::Committed(_) => Ok(()),
        Commit::NotPublished { cause, stage } => Err(supplier(Failure::NotPublished {
            cause,
            cleanup: stage.abort().err(),
        })),
        Commit::PublishedUnconfirmed {
            receipt,
            phase,
            cause,
            sync_error,
        } => Err(supplier(Failure::Unconfirmed {
            receipt,
            phase,
            cause,
            sync_error,
        })),
    }
}
