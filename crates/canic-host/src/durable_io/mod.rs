//! Module: durable_io
//!
//! Responsibility: publish one complete host-owned file without exposing partial bytes.
//! Does not own: document serialization, multi-file transactions, or path selection.
//! Boundary: callers provide final bytes; this module owns sibling staging and filesystem syncs.

#[cfg(test)]
mod tests;

use std::{io, path::Path};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileCommitMode {
    Replace,
    CreateNew,
    CreateNewWithParents,
}

/// Durably replace one file through atomic publication of complete bytes.
///
/// Missing parent directories are created and durably linked before the file
/// is published. Serialization must complete before calling this helper.
pub fn write_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    commit_bytes(path, bytes, FileCommitMode::Replace)
}

/// Durably create one file without replacing an existing destination.
///
/// The parent directory must already exist. Serialization must complete before
/// calling this helper.
pub fn create_new_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    commit_bytes(path, bytes, FileCommitMode::CreateNew)
}

/// Durably create one file and its missing parent hierarchy without replacing
/// an existing destination.
pub fn create_new_bytes_with_parents(path: &Path, bytes: &[u8]) -> io::Result<()> {
    commit_bytes(path, bytes, FileCommitMode::CreateNewWithParents)
}

fn commit_bytes(path: &Path, bytes: &[u8], mode: FileCommitMode) -> io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
    {
        supported::commit_with_hook(path, bytes, mode, |_, _| Ok(()))
    }

    #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
    {
        let _ = (path, bytes, mode);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "durable atomic file publication is unsupported on platform {}",
                std::env::consts::OS
            ),
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
mod supported {
    use super::FileCommitMode;

    use std::{
        ffi::{OsStr, OsString},
        fs,
        io::{self, Write},
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use rustix::{
        fd::{AsFd, OwnedFd},
        fs::{self as unix_fs, AtFlags, Mode, OFlags, RenameFlags},
    };

    const TEMP_ATTEMPTS: usize = 64;
    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(super) enum FileCommitStep {
        ParentDirectoryCreate,
        CreatedDirectorySync,
        CreatedDirectoryParentSync,
        TemporaryFileCreate,
        TemporaryFileWrite,
        TemporaryFileSync,
        Publication,
        FinalParentSync,
    }

    pub(super) fn commit_with_hook(
        path: &Path,
        bytes: &[u8],
        mode: FileCommitMode,
        mut before: impl FnMut(FileCommitStep, &Path) -> io::Result<()>,
    ) -> io::Result<()> {
        let (parent, file_name) = split_target(path)?;
        if matches!(
            mode,
            FileCommitMode::Replace | FileCommitMode::CreateNewWithParents
        ) {
            create_parent_hierarchy(parent, &mut before)?;
        }
        let parent_fd = open_directory(parent)?;
        let (temp_name, temp_path, mut temp_file) =
            create_sibling_temp(&parent_fd, parent, file_name, &mut before)?;

        let staged = (|| {
            before(FileCommitStep::TemporaryFileWrite, &temp_path)?;
            temp_file.write_all(bytes)?;
            before(FileCommitStep::TemporaryFileSync, &temp_path)?;
            temp_file.sync_all()
        })();
        drop(temp_file);
        if let Err(error) = staged {
            remove_temp(&parent_fd, &temp_name);
            return Err(error);
        }

        if let Err(error) = before(FileCommitStep::Publication, path) {
            remove_temp(&parent_fd, &temp_name);
            return Err(error);
        }
        let published = match mode {
            FileCommitMode::Replace => {
                unix_fs::renameat(&parent_fd, &temp_name, &parent_fd, file_name)
            }
            FileCommitMode::CreateNew | FileCommitMode::CreateNewWithParents => {
                unix_fs::renameat_with(
                    &parent_fd,
                    &temp_name,
                    &parent_fd,
                    file_name,
                    RenameFlags::NOREPLACE,
                )
            }
        };
        if let Err(error) = published {
            remove_temp(&parent_fd, &temp_name);
            return Err(errno_to_io(error));
        }

        before(FileCommitStep::FinalParentSync, parent)?;
        unix_fs::fsync(&parent_fd).map_err(errno_to_io)
    }

    fn split_target(path: &Path) -> io::Result<(&Path, &OsStr)> {
        let file_name = path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("durable write target has no file name: {}", path.display()),
            )
        })?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        Ok((parent, file_name))
    }

    fn create_parent_hierarchy(
        parent: &Path,
        before: &mut impl FnMut(FileCommitStep, &Path) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut missing = Vec::new();
        let mut current = parent;
        loop {
            match fs::symlink_metadata(current) {
                Ok(metadata) if metadata.is_dir() => break,
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::NotADirectory,
                        format!("output parent is not a directory: {}", current.display()),
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    missing.push(current.to_path_buf());
                    current = current
                        .parent()
                        .filter(|ancestor| !ancestor.as_os_str().is_empty())
                        .unwrap_or_else(|| Path::new("."));
                }
                Err(error) => return Err(error),
            }
        }

        for directory in missing.into_iter().rev() {
            before(FileCommitStep::ParentDirectoryCreate, &directory)?;
            match fs::create_dir(&directory) {
                Ok(()) => sync_created_directory(&directory, before)?,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if !fs::symlink_metadata(&directory)?.is_dir() {
                        return Err(io::Error::new(
                            io::ErrorKind::NotADirectory,
                            format!("output parent is not a directory: {}", directory.display()),
                        ));
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn sync_created_directory(
        directory: &Path,
        before: &mut impl FnMut(FileCommitStep, &Path) -> io::Result<()>,
    ) -> io::Result<()> {
        before(FileCommitStep::CreatedDirectorySync, directory)?;
        let directory_fd = open_directory(directory)?;
        unix_fs::fsync(&directory_fd).map_err(errno_to_io)?;

        let owner = directory
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        before(FileCommitStep::CreatedDirectoryParentSync, owner)?;
        let owner_fd = open_directory(owner)?;
        unix_fs::fsync(&owner_fd).map_err(errno_to_io)
    }

    fn create_sibling_temp(
        parent_fd: &impl AsFd,
        parent: &Path,
        file_name: &OsStr,
        before: &mut impl FnMut(FileCommitStep, &Path) -> io::Result<()>,
    ) -> io::Result<(OsString, PathBuf, fs::File)> {
        for _ in 0..TEMP_ATTEMPTS {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let mut temp_name = OsString::from(".");
            temp_name.push(file_name);
            temp_name.push(format!(".canic-tmp-{}-{sequence}", std::process::id()));
            let temp_path = parent.join(&temp_name);
            before(FileCommitStep::TemporaryFileCreate, &temp_path)?;
            match unix_fs::openat(
                parent_fd,
                &temp_name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o666),
            ) {
                Ok(file) => return Ok((temp_name, temp_path, fs::File::from(file))),
                Err(error) if error == rustix::io::Errno::EXIST => {}
                Err(error) => return Err(errno_to_io(error)),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "could not allocate a unique sibling temporary file for {}",
                parent.join(file_name).display()
            ),
        ))
    }

    fn open_directory(path: &Path) -> io::Result<OwnedFd> {
        unix_fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(errno_to_io)
    }

    fn remove_temp(parent_fd: &impl AsFd, temp_name: &OsStr) {
        let _ = unix_fs::unlinkat(parent_fd, temp_name, AtFlags::empty());
    }

    fn errno_to_io(error: rustix::io::Errno) -> io::Error {
        io::Error::from_raw_os_error(error.raw_os_error())
    }
}
