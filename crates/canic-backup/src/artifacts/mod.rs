//! Module: artifacts
//!
//! Responsibility: compute and validate backup artifact checksums.
//! Does not own: snapshot capture, artifact storage, or restore planning.
//! Boundary: provides deterministic checksum primitives to backup workflows.

#[cfg(test)]
mod tests;

use crate::hash::{hex_bytes, sha256_hex};

use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

const SHA256_ALGORITHM: &str = "sha256";

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
        let mut file = File::open(path)?;
        Self::from_reader(&mut file)
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

    /// Compute a SHA-256 checksum from a file or deterministic directory listing.
    pub fn from_path(path: &Path) -> Result<Self, ArtifactChecksumError> {
        if path.is_dir() {
            Self::from_directory(path)
        } else {
            Self::from_file(path)
        }
    }

    /// Compute a deterministic SHA-256 checksum over all files in a directory.
    pub fn from_directory(path: &Path) -> Result<Self, ArtifactChecksumError> {
        let mut files = Vec::new();
        collect_files(path, path, &mut files)?;
        files.sort();

        let checksums = files
            .into_iter()
            .map(|relative_path| {
                let checksum = Self::from_file(&path.join(&relative_path))?;
                Ok((relative_path, checksum))
            })
            .collect::<Result<Vec<_>, ArtifactChecksumError>>()?;
        Ok(Self::from_relative_file_checksums(checksums))
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
        if self.algorithm != SHA256_ALGORITHM {
            return Err(ArtifactChecksumError::UnsupportedAlgorithm(
                self.algorithm.clone(),
            ));
        }

        if self.hash == expected_hash {
            Ok(())
        } else {
            Err(ArtifactChecksumError::ChecksumMismatch {
                expected: expected_hash.to_string(),
                actual: self.hash.clone(),
            })
        }
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
    Io(#[from] io::Error),

    #[error("unsupported checksum algorithm {0}")]
    UnsupportedAlgorithm(String),

    #[error("checksum mismatch: expected {expected}, actual {actual}")]
    ChecksumMismatch { expected: String, actual: String },
}

// Recursively collect file paths relative to a directory root.
fn collect_files(
    root: &Path,
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), ArtifactChecksumError> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(io::Error::other)?
                .to_path_buf();
            files.push(relative);
        }
    }
    Ok(())
}
