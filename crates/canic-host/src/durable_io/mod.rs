//! Module: durable_io
//!
//! Responsibility: own atomic durable regular-file publication and narrow canonical-document
//! mechanics for `canic-host`.
//! Does not own: domain schemas or transitions, path selection, ephemeral protocol files, open
//! command-result descriptors, backup persistence, or multi-file transactions.
//! Boundary: reads reject links/special files; writes own sibling staging, publication, cleanup,
//! and filesystem syncs behind replace and create-new modes; document helpers add only bounded
//! encoding, reads and exact replacement reconciliation.

#[cfg(test)]
mod tests;

use std::{fs, io, path::Path};

#[derive(Debug)]
pub(crate) enum RegularFileReadError {
    NotRegular,
    Io(io::Error),
    #[cfg(not(unix))]
    UnsupportedPlatform,
}

#[derive(Debug)]
pub(crate) enum BoundedRegularFileReadError {
    Read(RegularFileReadError),
    TooLarge,
}

#[derive(Debug)]
pub(crate) enum RegularFileLockError {
    NotRegular,
    Io(io::Error),
    #[cfg(windows)]
    UnsupportedPlatform,
}

/// Read one optional regular file without following a final symlink.
pub(crate) fn read_optional_regular_bytes(
    path: &Path,
) -> Result<Option<Vec<u8>>, RegularFileReadError> {
    #[cfg(unix)]
    {
        supported::read_optional_regular_bytes(path)
    }

    #[cfg(not(unix))]
    {
        let _ = path;
        Err(RegularFileReadError::UnsupportedPlatform)
    }
}

/// Read at most `maximum_bytes + 1` from one optional regular no-follow file.
///
/// Descriptor metadata rejects an already oversized file before allocating its
/// contents. The extra byte in the bounded read detects growth after that
/// metadata observation.
pub(crate) fn read_optional_regular_bytes_bounded(
    path: &Path,
    maximum_bytes: usize,
) -> Result<Option<Vec<u8>>, BoundedRegularFileReadError> {
    #[cfg(unix)]
    {
        supported::read_optional_regular_bytes_bounded(path, maximum_bytes)
    }

    #[cfg(not(unix))]
    {
        let _ = (path, maximum_bytes);
        Err(BoundedRegularFileReadError::Read(
            RegularFileReadError::UnsupportedPlatform,
        ))
    }
}

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

/// Open and exclusively lock one durable regular no-follow file.
///
/// The lock file and missing parent hierarchy are durably created first. The
/// returned descriptor owns the kernel lock and is close-on-exec.
pub(crate) fn lock_regular_file_with_parents(
    path: &Path,
) -> Result<fs::File, RegularFileLockError> {
    match create_new_bytes_with_parents(path, &[]) {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
        Err(source) => return Err(RegularFileLockError::Io(source)),
    }
    let metadata = fs::symlink_metadata(path).map_err(RegularFileLockError::Io)?;
    if !metadata.file_type().is_file() {
        return Err(RegularFileLockError::NotRegular);
    }

    #[cfg(not(windows))]
    {
        use rustix::{
            fd::OwnedFd,
            fs::{FileType, FlockOperation, Mode, OFlags, flock, fstat, open},
        };

        let fd: OwnedFd = open(
            path,
            OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(errno_to_lock_error)?;
        let metadata = fstat(&fd).map_err(errno_to_lock_error)?;
        if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile {
            return Err(RegularFileLockError::NotRegular);
        }
        let file = fs::File::from(fd);
        flock(&file, FlockOperation::LockExclusive).map_err(errno_to_lock_error)?;
        Ok(file)
    }

    #[cfg(windows)]
    {
        Err(RegularFileLockError::UnsupportedPlatform)
    }
}

#[cfg(not(windows))]
fn errno_to_lock_error(source: rustix::io::Errno) -> RegularFileLockError {
    RegularFileLockError::Io(io::Error::from_raw_os_error(source.raw_os_error()))
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
    use super::{BoundedRegularFileReadError, FileCommitMode, RegularFileReadError};

    use std::{
        ffi::{OsStr, OsString},
        fs,
        io::{self, Read, Write},
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use rustix::{
        fd::{AsFd, OwnedFd},
        fs::{self as unix_fs, AtFlags, Mode, OFlags, RenameFlags},
    };

    const TEMP_ATTEMPTS: usize = 64;
    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    pub(super) fn read_optional_regular_bytes(
        path: &Path,
    ) -> Result<Option<Vec<u8>>, RegularFileReadError> {
        let Some((mut file, _)) = open_optional_regular_file(path)? else {
            return Ok(None);
        };
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(RegularFileReadError::Io)?;
        Ok(Some(bytes))
    }

    pub(super) fn read_optional_regular_bytes_bounded(
        path: &Path,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, BoundedRegularFileReadError> {
        read_optional_regular_bytes_bounded_with_hook(path, maximum_bytes, || Ok(()))
    }

    #[cfg(test)]
    pub(super) fn read_optional_regular_bytes_bounded_with_hook(
        path: &Path,
        maximum_bytes: usize,
        before_read: impl FnOnce() -> io::Result<()>,
    ) -> Result<Option<Vec<u8>>, BoundedRegularFileReadError> {
        read_optional_regular_bytes_bounded_inner(path, maximum_bytes, before_read)
    }

    #[cfg(not(test))]
    fn read_optional_regular_bytes_bounded_with_hook(
        path: &Path,
        maximum_bytes: usize,
        before_read: impl FnOnce() -> io::Result<()>,
    ) -> Result<Option<Vec<u8>>, BoundedRegularFileReadError> {
        read_optional_regular_bytes_bounded_inner(path, maximum_bytes, before_read)
    }

    fn read_optional_regular_bytes_bounded_inner(
        path: &Path,
        maximum_bytes: usize,
        before_read: impl FnOnce() -> io::Result<()>,
    ) -> Result<Option<Vec<u8>>, BoundedRegularFileReadError> {
        let Some((mut file, observed_size)) =
            open_optional_regular_file(path).map_err(BoundedRegularFileReadError::Read)?
        else {
            return Ok(None);
        };
        if observed_size > u64::try_from(maximum_bytes).unwrap_or(u64::MAX) {
            return Err(BoundedRegularFileReadError::TooLarge);
        }
        before_read().map_err(|source| {
            BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))
        })?;

        let read_limit = maximum_bytes
            .checked_add(1)
            .ok_or(BoundedRegularFileReadError::TooLarge)?;
        let mut bytes = Vec::with_capacity(
            usize::try_from(observed_size)
                .unwrap_or(maximum_bytes)
                .min(maximum_bytes),
        );
        Read::by_ref(&mut file)
            .take(u64::try_from(read_limit).unwrap_or(u64::MAX))
            .read_to_end(&mut bytes)
            .map_err(RegularFileReadError::Io)
            .map_err(BoundedRegularFileReadError::Read)?;
        if bytes.len() > maximum_bytes {
            return Err(BoundedRegularFileReadError::TooLarge);
        }
        Ok(Some(bytes))
    }

    fn open_optional_regular_file(
        path: &Path,
    ) -> Result<Option<(fs::File, u64)>, RegularFileReadError> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(RegularFileReadError::Io(error)),
        };
        if !metadata.file_type().is_file() {
            return Err(RegularFileReadError::NotRegular);
        }

        let fd: OwnedFd = unix_fs::open(
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(errno_to_io)
        .map_err(RegularFileReadError::Io)?;
        let metadata = unix_fs::fstat(&fd)
            .map_err(errno_to_io)
            .map_err(RegularFileReadError::Io)?;
        if unix_fs::FileType::from_raw_mode(metadata.st_mode) != unix_fs::FileType::RegularFile {
            return Err(RegularFileReadError::NotRegular);
        }

        let size = u64::try_from(metadata.st_size).map_err(|_| {
            RegularFileReadError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "regular file has a negative size",
            ))
        })?;
        Ok(Some((fs::File::from(fd), size)))
    }

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
                publish_create_new(&parent_fd, &temp_name, file_name)
            }
        };
        if let Err(error) = published {
            remove_temp(&parent_fd, &temp_name);
            return Err(errno_to_io(error));
        }

        before(FileCommitStep::FinalParentSync, parent)?;
        unix_fs::fsync(&parent_fd).map_err(errno_to_io)
    }

    fn publish_create_new(
        parent_fd: &OwnedFd,
        temp_name: &OsStr,
        file_name: &OsStr,
    ) -> rustix::io::Result<()> {
        let renamed = unix_fs::renameat_with(
            parent_fd,
            temp_name,
            parent_fd,
            file_name,
            RenameFlags::NOREPLACE,
        );
        finish_create_new_publication(parent_fd, temp_name, file_name, renamed)
    }

    fn finish_create_new_publication(
        parent_fd: &OwnedFd,
        temp_name: &OsStr,
        file_name: &OsStr,
        renamed: rustix::io::Result<()>,
    ) -> rustix::io::Result<()> {
        match renamed {
            Ok(()) => Ok(()),
            Err(
                rustix::io::Errno::INVAL | rustix::io::Errno::NOSYS | rustix::io::Errno::OPNOTSUPP,
            ) => {
                unix_fs::linkat(parent_fd, temp_name, parent_fd, file_name, AtFlags::empty())?;
                remove_temp(parent_fd, temp_name);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    #[cfg(test)]
    pub(super) fn publish_create_new_after_error(
        parent: &Path,
        temp_name: &OsStr,
        file_name: &OsStr,
        error: rustix::io::Errno,
    ) -> io::Result<()> {
        let parent_fd = open_directory(parent)?;
        finish_create_new_publication(&parent_fd, temp_name, file_name, Err(error))
            .map_err(errno_to_io)
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
