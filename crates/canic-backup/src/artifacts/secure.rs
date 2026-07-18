//! Module: artifacts::secure
//!
//! Responsibility: traverse and stage artifact trees without following symlinks.
//! Does not own: backup-root selection, manifest authority, or restore execution.
//! Boundary: returns checksums for the exact descriptor-read bytes.

use super::{ArtifactChecksum, ArtifactChecksumError};

use std::path::Path;

#[derive(Clone, Copy)]
pub(super) enum ExpectedArtifactType {
    Any,
    Directory,
    File,
}

pub(super) fn checksum_path(
    path: &Path,
    expected: ExpectedArtifactType,
) -> Result<ArtifactChecksum, ArtifactChecksumError> {
    #[cfg(unix)]
    {
        unix::checksum_path(path, expected)
    }

    #[cfg(not(unix))]
    {
        let _ = (path, expected);
        Err(ArtifactChecksumError::UnsupportedPlatform(
            std::env::consts::OS,
        ))
    }
}

pub(super) fn checksum_relative_path(
    root: &Path,
    relative: &Path,
) -> Result<ArtifactChecksum, ArtifactChecksumError> {
    #[cfg(unix)]
    {
        unix::checksum_relative_path(root, relative)
    }

    #[cfg(not(unix))]
    {
        let _ = (root, relative);
        Err(ArtifactChecksumError::UnsupportedPlatform(
            std::env::consts::OS,
        ))
    }
}

pub(super) fn stage_relative_path(
    root: &Path,
    relative: &Path,
    destination: &Path,
) -> Result<ArtifactChecksum, ArtifactChecksumError> {
    #[cfg(unix)]
    {
        unix::stage_relative_path(root, relative, destination)
    }

    #[cfg(not(unix))]
    {
        let _ = (root, relative, destination);
        Err(ArtifactChecksumError::UnsupportedPlatform(
            std::env::consts::OS,
        ))
    }
}

#[cfg(unix)]
mod unix {
    use super::{ArtifactChecksum, ArtifactChecksumError, ExpectedArtifactType};

    use std::{
        ffi::OsStr,
        fs::{DirBuilder, File, OpenOptions},
        os::unix::{
            ffi::OsStrExt,
            fs::{DirBuilderExt, OpenOptionsExt},
        },
        path::{Component, Path, PathBuf},
    };

    use rustix::{
        fd::{AsFd, OwnedFd},
        fs::{self as unix_fs, AtFlags, Dir, FileType, Mode, OFlags},
    };

    pub(super) fn checksum_path(
        path: &Path,
        expected: ExpectedArtifactType,
    ) -> Result<ArtifactChecksum, ArtifactChecksumError> {
        let (root, relative) = if path.is_absolute() {
            (
                Path::new("/"),
                path.strip_prefix("/").map_err(std::io::Error::other)?,
            )
        } else {
            (Path::new("."), path)
        };
        let (artifact, kind) = open_relative(root, relative)?;
        match expected {
            ExpectedArtifactType::Any => {}
            ExpectedArtifactType::Directory if kind == FileType::Directory => {}
            ExpectedArtifactType::File if kind == FileType::RegularFile => {}
            ExpectedArtifactType::Directory | ExpectedArtifactType::File => {
                return Err(unsupported_entry(path, kind));
            }
        }
        checksum_opened(&artifact, kind, path)
    }

    pub(super) fn checksum_relative_path(
        root: &Path,
        relative: &Path,
    ) -> Result<ArtifactChecksum, ArtifactChecksumError> {
        let (artifact, kind) = open_relative(root, relative)?;
        checksum_opened(&artifact, kind, relative)
    }

    pub(super) fn stage_relative_path(
        root: &Path,
        relative: &Path,
        destination: &Path,
    ) -> Result<ArtifactChecksum, ArtifactChecksumError> {
        let (artifact, kind) = open_relative(root, relative)?;
        match kind {
            FileType::RegularFile => copy_file(&artifact, destination),
            FileType::Directory => {
                create_private_directory(destination)?;
                copy_directory(&artifact, Path::new(""), relative, destination)
            }
            kind => Err(unsupported_entry(relative, kind)),
        }
    }

    fn open_relative(
        root: &Path,
        relative: &Path,
    ) -> Result<(OwnedFd, FileType), ArtifactChecksumError> {
        let components = relative
            .components()
            .map(|component| match component {
                Component::Normal(name) => Ok(name),
                _ => Err(unsupported_entry(relative, FileType::Unknown)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if components.is_empty() {
            return Err(unsupported_entry(relative, FileType::Unknown));
        }

        let mut current = unix_fs::open(
            root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(errno_to_io)?;
        for (index, component) in components.iter().enumerate() {
            let is_leaf = index + 1 == components.len();
            let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
            if !is_leaf {
                flags |= OFlags::DIRECTORY;
            }
            current =
                unix_fs::openat(&current, *component, flags, Mode::empty()).map_err(errno_to_io)?;
            if !is_leaf {
                ensure_opened_type(&current, FileType::Directory, relative)?;
            }
        }
        let metadata = unix_fs::fstat(&current).map_err(errno_to_io)?;
        Ok((current, FileType::from_raw_mode(metadata.st_mode)))
    }

    fn checksum_opened(
        artifact: &OwnedFd,
        kind: FileType,
        display_path: &Path,
    ) -> Result<ArtifactChecksum, ArtifactChecksumError> {
        match kind {
            FileType::RegularFile => {
                let mut file = File::from(artifact.try_clone()?);
                ArtifactChecksum::from_reader(&mut file)
            }
            FileType::Directory => checksum_directory(artifact, Path::new(""), display_path),
            kind => Err(unsupported_entry(display_path, kind)),
        }
    }

    fn checksum_directory(
        directory_fd: &OwnedFd,
        relative_directory: &Path,
        display_root: &Path,
    ) -> Result<ArtifactChecksum, ArtifactChecksumError> {
        let mut checksums = Vec::new();
        collect_directory_checksums(
            directory_fd,
            relative_directory,
            display_root,
            &mut checksums,
        )?;
        Ok(ArtifactChecksum::from_relative_file_checksums(checksums))
    }

    fn collect_directory_checksums(
        directory_fd: &OwnedFd,
        relative_directory: &Path,
        display_root: &Path,
        checksums: &mut Vec<(PathBuf, ArtifactChecksum)>,
    ) -> Result<(), ArtifactChecksumError> {
        let mut directory = Dir::read_from(directory_fd).map_err(errno_to_io)?;
        while let Some(entry) = directory.read() {
            let entry = entry.map_err(errno_to_io)?;
            let name_bytes = entry.file_name().to_bytes();
            if matches!(name_bytes, b"." | b"..") {
                continue;
            }
            let name = OsStr::from_bytes(name_bytes);
            let relative_path = relative_directory.join(name);
            let display_path = display_root.join(&relative_path);
            let kind = entry_type(directory_fd, entry.file_name())?;
            match kind {
                FileType::RegularFile => {
                    let child = open_child(directory_fd, entry.file_name(), false)?;
                    ensure_opened_type(&child, FileType::RegularFile, &display_path)?;
                    let mut file = File::from(child);
                    checksums.push((relative_path, ArtifactChecksum::from_reader(&mut file)?));
                }
                FileType::Directory => {
                    let child = open_child(directory_fd, entry.file_name(), true)?;
                    ensure_opened_type(&child, FileType::Directory, &display_path)?;
                    collect_directory_checksums(&child, &relative_path, display_root, checksums)?;
                }
                kind => return Err(unsupported_entry(&display_path, kind)),
            }
        }
        Ok(())
    }

    fn copy_directory(
        directory_fd: &OwnedFd,
        relative_directory: &Path,
        display_root: &Path,
        destination_root: &Path,
    ) -> Result<ArtifactChecksum, ArtifactChecksumError> {
        let mut checksums = Vec::new();
        copy_directory_entries(
            directory_fd,
            relative_directory,
            display_root,
            destination_root,
            &mut checksums,
        )?;
        Ok(ArtifactChecksum::from_relative_file_checksums(checksums))
    }

    fn copy_directory_entries(
        directory_fd: &OwnedFd,
        relative_directory: &Path,
        display_root: &Path,
        destination_root: &Path,
        checksums: &mut Vec<(PathBuf, ArtifactChecksum)>,
    ) -> Result<(), ArtifactChecksumError> {
        let mut directory = Dir::read_from(directory_fd).map_err(errno_to_io)?;
        while let Some(entry) = directory.read() {
            let entry = entry.map_err(errno_to_io)?;
            let name_bytes = entry.file_name().to_bytes();
            if matches!(name_bytes, b"." | b"..") {
                continue;
            }
            let name = OsStr::from_bytes(name_bytes);
            let relative_path = relative_directory.join(name);
            let display_path = display_root.join(&relative_path);
            let destination = destination_root.join(&relative_path);
            let kind = entry_type(directory_fd, entry.file_name())?;
            match kind {
                FileType::RegularFile => {
                    let child = open_child(directory_fd, entry.file_name(), false)?;
                    ensure_opened_type(&child, FileType::RegularFile, &display_path)?;
                    checksums.push((relative_path, copy_file(&child, &destination)?));
                }
                FileType::Directory => {
                    let child = open_child(directory_fd, entry.file_name(), true)?;
                    ensure_opened_type(&child, FileType::Directory, &display_path)?;
                    create_private_directory(&destination)?;
                    copy_directory_entries(
                        &child,
                        &relative_path,
                        display_root,
                        destination_root,
                        checksums,
                    )?;
                }
                kind => return Err(unsupported_entry(&display_path, kind)),
            }
        }
        Ok(())
    }

    fn copy_file(
        source: &OwnedFd,
        destination: &Path,
    ) -> Result<ArtifactChecksum, ArtifactChecksumError> {
        let mut source = File::from(source.try_clone()?);
        let mut destination = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(destination)?;
        ArtifactChecksum::copy_from_reader(&mut source, &mut destination)
    }

    fn create_private_directory(path: &Path) -> Result<(), ArtifactChecksumError> {
        DirBuilder::new().mode(0o700).create(path)?;
        Ok(())
    }

    fn entry_type(
        parent: &impl AsFd,
        name: &std::ffi::CStr,
    ) -> Result<FileType, ArtifactChecksumError> {
        let metadata =
            unix_fs::statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(errno_to_io)?;
        Ok(FileType::from_raw_mode(metadata.st_mode))
    }

    fn open_child(
        parent: &impl AsFd,
        name: &std::ffi::CStr,
        directory: bool,
    ) -> Result<OwnedFd, ArtifactChecksumError> {
        let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
        if directory {
            flags |= OFlags::DIRECTORY;
        }
        unix_fs::openat(parent, name, flags, Mode::empty())
            .map_err(errno_to_io)
            .map_err(ArtifactChecksumError::from)
    }

    fn ensure_opened_type(
        fd: &impl AsFd,
        expected: FileType,
        path: &Path,
    ) -> Result<(), ArtifactChecksumError> {
        let metadata = unix_fs::fstat(fd).map_err(errno_to_io)?;
        let actual = FileType::from_raw_mode(metadata.st_mode);
        if actual == expected {
            Ok(())
        } else {
            Err(unsupported_entry(path, actual))
        }
    }

    fn unsupported_entry(path: &Path, kind: FileType) -> ArtifactChecksumError {
        ArtifactChecksumError::UnsupportedEntry {
            path: path.display().to_string(),
            kind: format!("{kind:?}"),
        }
    }

    fn errno_to_io(error: rustix::io::Errno) -> std::io::Error {
        std::io::Error::from_raw_os_error(error.raw_os_error())
    }
}
