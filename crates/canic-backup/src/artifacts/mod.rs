//! Module: artifacts
//!
//! Responsibility: derive artifact path identities and compute or validate checksums.
//! Does not own: snapshot capture, artifact storage, or restore planning.
//! Boundary: provides deterministic checksum primitives to backup workflows.

mod secure;
#[cfg(test)]
mod tests;

use crate::hash::{hex_bytes, sha256_hex};

use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
        Self {
            algorithm: SHA256_ALGORITHM.to_string(),
            hash: sha256_hex(bytes),
        }
    }

    /// Compute a SHA-256 checksum from one filesystem file.
    pub fn from_file(path: &Path) -> Result<Self, ArtifactChecksumError> {
        secure::checksum_path(path, secure::ExpectedArtifactType::File)
    }

    /// Compute a file checksum from an already-open artifact descriptor.
    pub(crate) fn from_reader(reader: &mut impl Read) -> Result<Self, ArtifactChecksumError> {
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];

        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }

        Ok(Self {
            algorithm: SHA256_ALGORITHM.to_string(),
            hash: hex_bytes(hasher.finalize()),
        })
    }

    pub(crate) fn copy_from_reader(
        reader: &mut impl Read,
        writer: &mut impl Write,
    ) -> Result<Self, ArtifactChecksumError> {
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];

        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            writer.write_all(&buffer[..read])?;
            hasher.update(&buffer[..read]);
        }

        Ok(Self {
            algorithm: SHA256_ALGORITHM.to_string(),
            hash: hex_bytes(hasher.finalize()),
        })
    }

    /// Compute a SHA-256 checksum from a file or deterministic directory listing.
    pub fn from_path(path: &Path) -> Result<Self, ArtifactChecksumError> {
        secure::checksum_path(path, secure::ExpectedArtifactType::Any)
    }

    /// Compute a deterministic SHA-256 checksum over all files in a directory.
    pub fn from_directory(path: &Path) -> Result<Self, ArtifactChecksumError> {
        secure::checksum_path(path, secure::ExpectedArtifactType::Directory)
    }

    /// Compose the maintained directory checksum from relative file checksums.
    pub(crate) fn from_relative_file_checksums(mut files: Vec<(PathBuf, Self)>) -> Self {
        files.sort_by(|left, right| left.0.cmp(&right.0));
        let mut hasher = Sha256::new();
        for (relative_path, file_checksum) in files {
            hasher.update(relative_path.to_string_lossy().as_bytes());
            hasher.update([0]);
            hasher.update(file_checksum.hash.as_bytes());
            hasher.update(*b"\n");
        }

        Self {
            algorithm: SHA256_ALGORITHM.to_string(),
            hash: hex_bytes(hasher.finalize()),
        }
    }

    /// Verify that the checksum matches an expected SHA-256 hash.
    pub fn verify(&self, expected_hash: &str) -> Result<(), ArtifactChecksumError> {
        self.validate()?;

        if self.hash == expected_hash {
            Ok(())
        } else {
            Err(ArtifactChecksumError::ChecksumMismatch {
                expected: expected_hash.to_string(),
                actual: self.hash.clone(),
            })
        }
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
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ArtifactChecksumError::InvalidHash(hash.to_string()));
        }

        Ok(())
    }

    pub(crate) fn from_relative_path_no_follow(
        root: &Path,
        relative: &Path,
    ) -> Result<Self, ArtifactChecksumError> {
        secure::checksum_relative_path(root, relative)
    }

    pub(crate) fn stage_relative_path_no_follow(
        root: &Path,
        relative: &Path,
        destination: &Path,
    ) -> Result<Self, ArtifactChecksumError> {
        secure::stage_relative_path(root, relative, destination)
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

    #[error("unsupported artifact filesystem entry at {path}: {kind}")]
    UnsupportedEntry { path: String, kind: String },

    #[error("secure artifact traversal is unsupported on platform {0}")]
    UnsupportedPlatform(&'static str),
}
