use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const SHA256_ALGORITHM: &str = "sha256";

///
/// ArtifactChecksum
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
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];

        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }

        Ok(Self {
            algorithm: SHA256_ALGORITHM.to_string(),
            hash: digest_hex(hasher.finalize()),
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

        let mut hasher = Sha256::new();
        for relative_path in files {
            let full_path = path.join(&relative_path);
            let file_checksum = Self::from_file(&full_path)?;
            hasher.update(relative_path.to_string_lossy().as_bytes());
            hasher.update([0]);
            hasher.update(file_checksum.hash.as_bytes());
            hasher.update([b'\n']);
        }

        Ok(Self {
            algorithm: SHA256_ALGORITHM.to_string(),
            hash: digest_hex(hasher.finalize()),
        })
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

// Compute lowercase hexadecimal SHA-256 from in-memory bytes.
fn sha256_hex(bytes: &[u8]) -> String {
    digest_hex(Sha256::digest(bytes))
}

// Encode a finalized digest as lowercase hexadecimal.
fn digest_hex(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(hex_char(byte >> 4));
        out.push(hex_char(byte & 0x0f));
    }
    out
}

// Convert one four-bit nibble to lowercase hexadecimal.
const fn hex_char(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    // Ensure empty-byte checksums match the standard SHA-256 vector.
    #[test]
    fn byte_checksum_matches_sha256_vector() {
        let checksum = ArtifactChecksum::from_bytes(&[]);

        assert_eq!(checksum.algorithm, "sha256");
        assert_eq!(checksum.hash, EMPTY_SHA256);
    }

    // Ensure file checksums use the same implementation as byte checksums.
    #[test]
    fn file_checksum_matches_byte_checksum() {
        let path = temp_path("canic-backup-checksum");
        fs::write(&path, b"canic backup artifact").expect("write temp artifact");

        let from_file = ArtifactChecksum::from_file(&path).expect("checksum file");
        let from_bytes = ArtifactChecksum::from_bytes(b"canic backup artifact");

        fs::remove_file(&path).expect("remove temp artifact");
        assert_eq!(from_file, from_bytes);
    }

    // Ensure directory checksums are stable regardless of file creation order.
    #[test]
    fn directory_checksum_is_order_independent() {
        let first = temp_path("canic-backup-dir-a");
        let second = temp_path("canic-backup-dir-b");
        fs::create_dir_all(first.join("nested")).expect("create first");
        fs::create_dir_all(second.join("nested")).expect("create second");

        fs::write(first.join("a.txt"), b"a").expect("write first a");
        fs::write(first.join("nested/b.txt"), b"b").expect("write first b");
        fs::write(second.join("nested/b.txt"), b"b").expect("write second b");
        fs::write(second.join("a.txt"), b"a").expect("write second a");

        let first_checksum = ArtifactChecksum::from_directory(&first).expect("checksum first");
        let second_checksum = ArtifactChecksum::from_directory(&second).expect("checksum second");

        fs::remove_dir_all(first).expect("remove first");
        fs::remove_dir_all(second).expect("remove second");
        assert_eq!(first_checksum, second_checksum);
    }

    // Ensure checksum verification reports mismatches.
    #[test]
    fn checksum_verify_rejects_mismatch() {
        let checksum = ArtifactChecksum::from_bytes(b"actual");

        let err = checksum
            .verify(EMPTY_SHA256)
            .expect_err("different hash should fail");

        assert!(matches!(
            err,
            ArtifactChecksumError::ChecksumMismatch { .. }
        ));
    }

    // Build a unique temp file path without adding test-only dependencies.
    fn temp_path(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
