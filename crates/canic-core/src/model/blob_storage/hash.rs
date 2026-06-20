//! Module: model::blob_storage::hash
//!
//! Responsibility: model the canonical Toko/Caffeine blob root hash string.
//! Does not own: DTO decoding, storage lookup, or gateway authorization.
//! Boundary: accepts only `sha256:<64-hex>` values and stores lowercase hex.

use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

pub const BLOB_ROOT_HASH_BYTE_LENGTH: usize = 32;

const BLOB_ROOT_HASH_PREFIX: &str = "sha256:";
const BLOB_ROOT_HASH_HEX_LENGTH: usize = BLOB_ROOT_HASH_BYTE_LENGTH * 2;
const BLOB_ROOT_HASH_TEXT_LENGTH: usize = BLOB_ROOT_HASH_PREFIX.len() + BLOB_ROOT_HASH_HEX_LENGTH;

///
/// BlobRootHash
///
/// Canonical Caffeine blob root identity used by Toko current-source
/// `blob_root_hash` fields and immutable object-storage gateway endpoints.
///

#[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BlobRootHash(String);

impl BlobRootHash {
    /// Return the canonical `sha256:<64-lowercase-hex>` text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the hash and return the canonical text.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Debug for BlobRootHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "BlobRootHash({self})")
    }
}

impl fmt::Display for BlobRootHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BlobRootHash {
    type Err = BlobRootHashError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        canonicalize_root_hash(value).map(Self)
    }
}

impl TryFrom<&str> for BlobRootHash {
    type Error = BlobRootHashError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for BlobRootHash {
    type Error = BlobRootHashError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

///
/// BlobRootHashError
///
/// Typed failure returned when parsing a canonical blob root hash.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlobRootHashError {
    Empty,
    InvalidPrefix,
    InvalidLength { actual: usize },
    InvalidHexCharacter { index: usize, byte: u8 },
}

impl fmt::Display for BlobRootHashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("blob root hash must not be empty"),
            Self::InvalidPrefix => {
                write!(
                    formatter,
                    "blob root hash must start with {BLOB_ROOT_HASH_PREFIX:?}"
                )
            }
            Self::InvalidLength { actual } => {
                write!(
                    formatter,
                    "blob root hash must be {BLOB_ROOT_HASH_TEXT_LENGTH} bytes, got {actual}"
                )
            }
            Self::InvalidHexCharacter { index, byte } => {
                write!(
                    formatter,
                    "blob root hash contains non-hex byte 0x{byte:02x} at byte index {index}"
                )
            }
        }
    }
}

impl Error for BlobRootHashError {}

fn canonicalize_root_hash(value: &str) -> Result<String, BlobRootHashError> {
    if value.is_empty() {
        return Err(BlobRootHashError::Empty);
    }
    if !value.starts_with(BLOB_ROOT_HASH_PREFIX) {
        return Err(BlobRootHashError::InvalidPrefix);
    }
    if value.len() != BLOB_ROOT_HASH_TEXT_LENGTH {
        return Err(BlobRootHashError::InvalidLength {
            actual: value.len(),
        });
    }

    let mut canonical = String::with_capacity(BLOB_ROOT_HASH_TEXT_LENGTH);
    canonical.push_str(BLOB_ROOT_HASH_PREFIX);
    for (offset, byte) in value.as_bytes()[BLOB_ROOT_HASH_PREFIX.len()..]
        .iter()
        .copied()
        .enumerate()
    {
        canonical.push(canonical_hex_char(
            byte,
            BLOB_ROOT_HASH_PREFIX.len() + offset,
        )?);
    }
    Ok(canonical)
}

fn canonical_hex_char(byte: u8, index: usize) -> Result<char, BlobRootHashError> {
    match byte {
        b'0'..=b'9' | b'a'..=b'f' => Ok(char::from(byte)),
        b'A'..=b'F' => Ok(char::from(byte + 32)),
        _ => Err(BlobRootHashError::InvalidHexCharacter { index, byte }),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const LOWER_HASH: &str =
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const UPPER_HASH: &str =
        "sha256:0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

    #[test]
    fn parses_canonical_toko_blob_root_hash() {
        let hash = BlobRootHash::try_from(LOWER_HASH).expect("hash parses");

        assert_eq!(hash.as_str(), LOWER_HASH);
        assert_eq!(hash.to_string(), LOWER_HASH);
        assert_eq!(hash.into_string(), LOWER_HASH);
    }

    #[test]
    fn normalizes_hex_digits_to_lowercase() {
        let hash = BlobRootHash::try_from(UPPER_HASH).expect("hash parses");

        assert_eq!(hash.as_str(), LOWER_HASH);
    }

    #[test]
    fn rejects_empty_hash() {
        assert_eq!(BlobRootHash::try_from(""), Err(BlobRootHashError::Empty));
    }

    #[test]
    fn rejects_wrong_prefix() {
        assert_eq!(
            BlobRootHash::try_from(
                "SHA256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            ),
            Err(BlobRootHashError::InvalidPrefix)
        );
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            BlobRootHash::try_from("sha256:00"),
            Err(BlobRootHashError::InvalidLength { actual: 9 })
        );
    }

    #[test]
    fn rejects_non_hex_text() {
        let value = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg";

        assert_eq!(
            BlobRootHash::try_from(value),
            Err(BlobRootHashError::InvalidHexCharacter {
                index: 70,
                byte: b'g',
            })
        );
    }
}
