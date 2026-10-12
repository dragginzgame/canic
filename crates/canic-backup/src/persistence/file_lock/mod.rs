//! Module: persistence::file_lock
//!
//! Responsibility: project Host sidecar admission and acquire command exclusion.
//! Does not own: file admission, journal or command-lifetime semantics, or paths.

use std::{fs, io, path::Path};

#[cfg(unix)]
use ic_host_fs::durable::{RegularFileLockError, open_private_regular_lock_file};
#[cfg(unix)]
use rustix::fs::{FlockOperation, flock};

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
fn acquire_supported(path: &Path) -> Result<fs::File, FileLockError> {
    let file = open_private_regular_lock_file(path).map_err(|error| match error {
        RegularFileLockError::NotRegular => FileLockError::UnsafeEntry {
            kind: "NonRegular".to_string(),
        },
        other => FileLockError::Io(io::Error::from(other)),
    })?;
    match flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => {}
        Err(error) if error == rustix::io::Errno::WOULDBLOCK => {
            return Err(FileLockError::Locked);
        }
        Err(error) => return Err(FileLockError::Io(io::Error::from(error))),
    }
    Ok(file)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::test_support::{temp_dir, temp_path};
    use rustix::fs::{CWD, Mode};
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

    #[test]
    fn command_sidecar_creation_keeps_owner_only_permissions() {
        let path = temp_path("canic-private-command-sidecar");
        let lock = acquire(&path).expect("acquire new sidecar");
        let metadata = lock.metadata().expect("read admitted descriptor");
        assert!(metadata.is_file());
        assert_eq!(metadata.permissions().mode() & 0o077, 0);
        assert_eq!(metadata.len(), 0);
        drop(lock);
        fs::remove_file(path).expect("remove sidecar");
    }

    #[test]
    fn missing_parent_refuses_without_creating_a_layout() {
        let root = temp_path("canic-missing-command-parent");
        let path = root.join("nested/command.lock");
        assert!(matches!(
            acquire(&path),
            Err(FileLockError::Io(error)) if error.kind() == io::ErrorKind::NotFound
        ));
        assert!(!root.exists());
    }

    #[test]
    fn existing_sidecar_preserves_bytes_inode_and_permissions_across_contention() {
        for mode in [0o600, 0o666] {
            let path = temp_path("canic-retained-command-sidecar");
            fs::write(&path, b"retained custody evidence").expect("write sidecar");
            fs::set_permissions(&path, fs::Permissions::from_mode(mode))
                .expect("set retained permissions");
            let before = fs::metadata(&path).expect("read original sidecar");
            let lock = acquire(&path).expect("acquire existing sidecar");
            assert!(matches!(acquire(&path), Err(FileLockError::Locked)));
            let after = fs::metadata(&path).expect("read contended sidecar");
            assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
            assert_eq!(after.permissions().mode(), before.permissions().mode());
            assert_eq!(fs::read(&path).unwrap(), b"retained custody evidence");
            drop(lock);
            drop(acquire(&path).expect("acquire after owner releases"));
            fs::remove_file(path).expect("remove sidecar");
        }
    }

    #[test]
    fn unsafe_sidecars_refuse_without_changing_the_entries_or_link_target() {
        let root = temp_dir("canic-unsafe-command-sidecars");
        fs::create_dir(&root).expect("create fixture directory");
        let target = root.join("target");
        fs::write(&target, b"original target").expect("write link target");
        let link = root.join("link");
        symlink(&target, &link).expect("create link");
        let directory = root.join("directory");
        fs::create_dir(&directory).expect("create directory");
        let fifo = root.join("fifo");
        rustix::fs::mkfifoat(CWD, &fifo, Mode::RUSR | Mode::WUSR).expect("create FIFO");
        for path in [&link, &directory, &fifo] {
            assert!(matches!(
                acquire(path),
                Err(FileLockError::UnsafeEntry { .. })
            ));
        }
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(directory.is_dir());
        assert_eq!(fs::read(&target).unwrap(), b"original target");
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
