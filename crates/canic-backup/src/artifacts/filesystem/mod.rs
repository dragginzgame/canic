//! Module: artifacts::filesystem
//!
//! Responsibility: hash and stage backup artifacts through retained no-follow descriptors.
//! Does not own: traversal mechanics, checksum framing, retry or restore orchestration.
//! Boundary: callers own stable source content and the private staging namespace.
use super::{
    ArtifactChecksumError, ArtifactChecksumRecord, ArtifactError, artifact_error,
    directory_artifact_error,
};
use ic_host_fs::read::{OpenEntryError, OpenedEntry, open_directory_no_follow, open_entry_beneath};
use rustix::fs::FileType;
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::{
        fd::{AsFd, OwnedFd},
        unix::fs::{DirBuilderExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

pub(super) fn relative(
    root: &Path,
    path: &Path,
    destination: Option<&Path>,
) -> Result<ArtifactChecksumRecord, ArtifactChecksumError> {
    let root = open_directory_no_follow(root)?;
    let entry = open_entry_beneath(root.as_fd(), path).map_err(|error| match error {
        OpenEntryError::Io(error) => ArtifactChecksumError::Io(error),
        OpenEntryError::UnsupportedEntry { kind } => {
            artifact_error(ArtifactError::UnsupportedEntry {
                path: path.display().to_string(),
                kind: format!("{kind:?}"),
            })
        }
    })?;
    match entry {
        OpenedEntry::RegularFile(mut file) => file_checksum(&mut file, destination),
        OpenedEntry::Directory(directory) => {
            let mut visitor = ArtifactVisitor {
                destination,
                checksums: Vec::new(),
            };
            if let Some(destination) = destination {
                create_directory(destination)?;
            }
            walk_tree(&OwnedFd::from(directory), Path::new(""), path, &mut visitor)?;
            ic_host_artifacts::artifact::tree::checksum_relative_files(
                visitor
                    .checksums
                    .into_iter()
                    .map(|(path, checksum)| {
                        checksum
                            .hash()
                            .parse()
                            .map(|digest| (path, digest))
                            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map(|digest| ArtifactChecksumRecord::from_digest(*digest.as_bytes()))
            .map_err(|error| artifact_error(directory_artifact_error(error)))
        }
    }
}

fn create_directory(path: &Path) -> io::Result<()> {
    fs::DirBuilder::new().mode(0o700).create(path)
}

fn file_checksum(
    file: &mut File,
    destination: Option<&Path>,
) -> Result<ArtifactChecksumRecord, ArtifactChecksumError> {
    let identity = if let Some(destination) = destination {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(destination)?;
        let identity = ic_host_artifacts::artifact::copy_reader(file, &mut output, u64::MAX)
            .map_err(io::Error::from)?;
        output.sync_all()?;
        identity
    } else {
        ic_host_artifacts::artifact::hash_reader(file, u64::MAX).map_err(io::Error::from)?
    };
    Ok(ArtifactChecksumRecord::from_digest(
        *identity.sha256.as_bytes(),
    ))
}

/// File identities and optional private destination for one admitted source tree.
struct ArtifactVisitor<'a> {
    destination: Option<&'a Path>,
    checksums: Vec<(PathBuf, ArtifactChecksumRecord)>,
}
impl TreeVisitor for ArtifactVisitor<'_> {
    type Error = ArtifactChecksumError;
    fn file(&mut self, fd: &OwnedFd, relative: &Path, _display: &Path) -> Result<(), Self::Error> {
        let mut file = File::from(fd.try_clone()?);
        let destination = self.destination.map(|root| root.join(relative));
        self.checksums.push((
            relative.to_path_buf(),
            file_checksum(&mut file, destination.as_deref())?,
        ));
        Ok(())
    }
    fn enter_directory(&mut self, relative: &Path, _display: &Path) -> Result<(), Self::Error> {
        if let Some(root) = self.destination {
            create_directory(&root.join(relative))?;
        }
        Ok(())
    }
    fn leave_directory(
        &mut self,
        _fd: &OwnedFd,
        relative: &Path,
        _display: &Path,
    ) -> Result<(), Self::Error> {
        if let Some(root) = self.destination {
            File::open(root.join(relative))?.sync_all()?;
        }
        Ok(())
    }
    fn unsupported(&self, display: &Path, kind: FileType) -> Self::Error {
        artifact_error(ArtifactError::UnsupportedEntry {
            path: display.display().to_string(),
            kind: format!("{kind:?}"),
        })
    }
    fn non_utf8(&self, display: &Path) -> Self::Error {
        artifact_error(ArtifactError::NonUtf8Path {
            path: display.to_path_buf(),
        })
    }
}

use ic_host_fs::read::tree::{TreeVisitor, walk_tree};
