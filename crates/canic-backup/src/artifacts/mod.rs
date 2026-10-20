//! Module: artifacts
//!
//! Responsibility: apply backup artifact filesystem policy using shared checksum records.
//! Does not own: snapshot capture, artifact storage, or restore planning.
//! Boundary: Host owns no-follow traversal and streaming; Backup owns checksum framing.

#[cfg(unix)]
mod filesystem;
#[cfg(test)]
mod tests;

#[cfg(test)]
use std::io::Read;
use std::{io, path::Path};

use ic_backup::{
    model::artifacts::{ArtifactChecksumRecord, ChecksumError},
    ops::artifacts::ArtifactError,
};
use ic_host_artifacts::artifact::tree::DirectoryChecksumError;
use thiserror::Error as ThisError;

/// Hash one file without following its final symlink.
pub fn checksum_file(path: &Path) -> Result<ArtifactChecksumRecord, ArtifactChecksumError> {
    ic_host_fs::read::hash_file_no_follow(path, u64::MAX)
        .map(|identity| ArtifactChecksumRecord::from_digest(*identity.sha256.as_bytes()))
        .map_err(|error| ArtifactChecksumError::Io(error.into()))
}

#[cfg(test)]
pub(crate) fn checksum_reader(
    reader: &mut impl Read,
) -> Result<ArtifactChecksumRecord, ArtifactChecksumError> {
    let identity =
        ic_host_artifacts::artifact::hash_reader(reader, u64::MAX).map_err(io::Error::from)?;
    Ok(ArtifactChecksumRecord::from_digest(
        *identity.sha256.as_bytes(),
    ))
}

/// Hash one file or deterministic directory tree.
pub fn checksum_path(path: &Path) -> Result<ArtifactChecksumRecord, ArtifactChecksumError> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        checksum_directory(path)
    } else {
        checksum_file(path)
    }
}

/// Hash a deterministic directory tree.
pub fn checksum_directory(path: &Path) -> Result<ArtifactChecksumRecord, ArtifactChecksumError> {
    #[cfg(unix)]
    {
        use ic_host_fs::read::tree::checksum::{TreeChecksumError, checksum_directory};
        let digest = checksum_directory(path, u64::MAX)
            .map_err(|error| match error {
                TreeChecksumError::Io(error) => ArtifactError::Io(error),
                TreeChecksumError::Input(error) => ArtifactError::Io(error.into()),
                TreeChecksumError::Declaration(error) => directory_artifact_error(error),
                TreeChecksumError::NonUtf8Path { path } => ArtifactError::NonUtf8Path { path },
                TreeChecksumError::InvalidPath { path } => ArtifactError::UnsupportedEntry {
                    path: path.display().to_string(),
                    kind: "Unknown".into(),
                },
                TreeChecksumError::UnsupportedEntry { path, kind } => {
                    ArtifactError::UnsupportedEntry {
                        path: path.display().to_string(),
                        kind: format!("{kind:?}"),
                    }
                }
            })
            .map_err(artifact_error)?;
        Ok(ArtifactChecksumRecord::from_digest(*digest.as_bytes()))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(artifact_error(ArtifactError::UnsupportedPlatform(
            std::env::consts::OS,
        )))
    }
}

pub(crate) fn checksum_relative_path(
    root: &Path,
    relative: &Path,
) -> Result<ArtifactChecksumRecord, ArtifactChecksumError> {
    #[cfg(unix)]
    {
        filesystem::relative(root, relative, None)
    }
    #[cfg(not(unix))]
    {
        let _ = (root, relative);
        Err(artifact_error(ArtifactError::UnsupportedPlatform(
            std::env::consts::OS,
        )))
    }
}

pub(crate) fn stage_relative_path(
    root: &Path,
    relative: &Path,
    destination: &Path,
) -> Result<ArtifactChecksumRecord, ArtifactChecksumError> {
    #[cfg(unix)]
    {
        filesystem::relative(root, relative, Some(destination))
    }
    #[cfg(not(unix))]
    {
        let _ = (root, relative, destination);
        Err(artifact_error(ArtifactError::UnsupportedPlatform(
            std::env::consts::OS,
        )))
    }
}

// Convert at Canic's boundary: Backup may select a different Host crate version.
fn directory_artifact_error(error: DirectoryChecksumError) -> ArtifactError {
    match error {
        DirectoryChecksumError::NonUtf8Path { path } => ArtifactError::NonUtf8Path { path },
        error => ArtifactError::Io(io::Error::new(io::ErrorKind::InvalidData, error)),
    }
}

fn artifact_error(error: ArtifactError) -> ArtifactChecksumError {
    match error {
        ArtifactError::Io(error) => ArtifactChecksumError::Io(error),
        error => ArtifactChecksumError::Artifact(error),
    }
}

///
/// ArtifactChecksumError
///
/// Typed checksum failure returned by backup artifact hashing and validation.
/// Owned by backup artifact support and surfaced to snapshot/runner callers.
///

#[derive(Debug, ThisError)]
pub enum ArtifactChecksumError {
    #[error(transparent)]
    Checksum(#[from] ChecksumError),

    #[error(transparent)]
    Io(#[from] io::Error),

    /// Shared no-follow traversal, path-identity or platform-admission failure.
    #[error(transparent)]
    Artifact(ArtifactError),
}
