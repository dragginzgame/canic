//! Module: artifacts
//!
//! Responsibility: project shared artifact checksums into Canic manifests and validate them.
//! Does not own: snapshot capture, artifact storage, or restore planning.
//! Boundary: IC Backup owns no-follow traversal, staging and publication; Canic owns manifests.

#[cfg(test)]
mod tests;

#[cfg(test)]
use std::io::Read;
use std::{io, path::Path};

use ic_backup::{
    model::artifacts::{ArtifactChecksumRecord, ChecksumError},
    ops::artifacts::{self, ArtifactError},
};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

const SHA256_ALGORITHM: &str = "sha256";

pub(crate) fn artifact_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

///
/// ArtifactChecksum
///
/// SHA-256 checksum metadata for a backup artifact file or directory.
/// Owned by backup artifact support and serialized into manifests.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactChecksum {
    pub algorithm: String,
    pub hash: String,
}

impl ArtifactChecksum {
    /// Compute a SHA-256 checksum from in-memory bytes.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_record(ArtifactChecksumRecord::from_bytes(bytes))
    }

    /// Compute a SHA-256 checksum from one filesystem file.
    pub fn from_file(path: &Path) -> Result<Self, ArtifactChecksumError> {
        artifacts::checksum_file(path)
            .map(Self::from_record)
            .map_err(artifact_error)
    }

    /// Compute a file checksum from an already-open artifact descriptor.
    #[cfg(test)]
    pub(crate) fn from_reader(reader: &mut impl Read) -> Result<Self, ArtifactChecksumError> {
        artifacts::checksum_reader(reader)
            .map(Self::from_record)
            .map_err(artifact_error)
    }

    /// Compute a SHA-256 checksum from a file or deterministic directory listing.
    pub fn from_path(path: &Path) -> Result<Self, ArtifactChecksumError> {
        artifacts::checksum_path(path)
            .map(Self::from_record)
            .map_err(artifact_error)
    }

    /// Compute a deterministic SHA-256 checksum over all files in a directory.
    pub fn from_directory(path: &Path) -> Result<Self, ArtifactChecksumError> {
        artifacts::checksum_directory(path)
            .map(Self::from_record)
            .map_err(artifact_error)
    }

    /// Verify that the checksum matches an expected SHA-256 hash.
    pub fn verify(&self, expected_hash: &str) -> Result<(), ArtifactChecksumError> {
        Self::validate_algorithm(&self.algorithm)?;
        ArtifactChecksumRecord::from_hash(&self.hash)
            .and_then(|record| record.verify(expected_hash))
            .map_err(|error| match error {
                ChecksumError::ChecksumMismatch { .. } => ArtifactChecksumError::ChecksumMismatch {
                    expected: expected_hash.to_string(),
                    actual: self.hash.clone(),
                },
                ChecksumError::InvalidHash(hash) => ArtifactChecksumError::InvalidHash(hash),
                ChecksumError::UnsupportedAlgorithm(algorithm) => {
                    ArtifactChecksumError::UnsupportedAlgorithm(algorithm)
                }
            })
    }

    /// Validate checksum metadata without comparing artifact bytes.
    pub fn validate(&self) -> Result<(), ArtifactChecksumError> {
        Self::validate_algorithm(&self.algorithm)?;
        Self::validate_hash(&self.hash)
    }

    /// Validate one artifact-checksum algorithm identifier.
    pub(crate) fn validate_algorithm(algorithm: &str) -> Result<(), ArtifactChecksumError> {
        if algorithm != SHA256_ALGORITHM {
            return Err(ArtifactChecksumError::UnsupportedAlgorithm(
                algorithm.to_string(),
            ));
        }

        Ok(())
    }

    /// Validate one artifact-checksum hash representation.
    pub(crate) fn validate_hash(hash: &str) -> Result<(), ArtifactChecksumError> {
        ArtifactChecksumRecord::from_hash(hash)
            .map(|_| ())
            .map_err(|_| ArtifactChecksumError::InvalidHash(hash.to_string()))
    }

    pub(crate) fn from_relative_path_no_follow(
        root: &Path,
        relative: &Path,
    ) -> Result<Self, ArtifactChecksumError> {
        artifacts::checksum_relative_path(root, relative)
            .map(Self::from_record)
            .map_err(artifact_error)
    }

    pub(crate) fn stage_relative_path_no_follow(
        root: &Path,
        relative: &Path,
        destination: &Path,
    ) -> Result<Self, ArtifactChecksumError> {
        artifacts::stage_relative_path(root, relative, destination)
            .map(Self::from_record)
            .map_err(artifact_error)
    }

    fn from_record(record: ArtifactChecksumRecord) -> Self {
        Self {
            algorithm: record.algorithm().to_owned(),
            hash: record.hash().to_owned(),
        }
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
    #[error("checksum mismatch: expected {expected}, actual {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("invalid SHA-256 checksum: {0}")]
    InvalidHash(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("unsupported checksum algorithm {0}")]
    UnsupportedAlgorithm(String),

    /// Shared no-follow traversal, path-identity or platform-admission failure.
    #[error(transparent)]
    Artifact(ArtifactError),
}
