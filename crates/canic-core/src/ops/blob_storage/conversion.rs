//! Module: ops::blob_storage::conversion
//!
//! Responsibility: convert boundary blob-storage hash inputs into model values.
//! Does not own: blob lifecycle storage, gateway-principal checks, or workflows.
//! Boundary: mirrors Toko 0.69 wire inputs while returning typed Canic errors.

use std::{error::Error, fmt};

use crate::{
    cdk::utils::hash::hex_bytes,
    model::blob_storage::{BLOB_ROOT_HASH_BYTE_LENGTH, BlobRootHash, BlobRootHashError},
};

///
/// BlobStorageConversionOps
///
/// Zero-cost namespace for blob-storage boundary conversions.
///

pub struct BlobStorageConversionOps;

impl BlobStorageConversionOps {
    /// Parse a Toko/Caffeine root hash string into canonical model form.
    pub fn root_hash_from_text(value: &str) -> Result<BlobRootHash, BlobStorageConversionError> {
        BlobRootHash::try_from(value).map_err(BlobStorageConversionError::InvalidRootHash)
    }

    /// Convert one gateway 32-byte root hash argument into canonical model form.
    pub fn root_hash_from_bytes(bytes: &[u8]) -> Result<BlobRootHash, BlobStorageConversionError> {
        if bytes.len() != BLOB_ROOT_HASH_BYTE_LENGTH {
            return Err(BlobStorageConversionError::InvalidRootHashByteLength {
                actual: bytes.len(),
            });
        }

        let value = format!("sha256:{}", hex_bytes(bytes));
        Self::root_hash_from_text(&value)
    }
}

///
/// BlobStorageConversionError
///
/// Typed failure returned by blob-storage boundary conversions.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlobStorageConversionError {
    InvalidRootHash(BlobRootHashError),
    InvalidRootHashByteLength { actual: usize },
}

impl fmt::Display for BlobStorageConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRootHash(err) => write!(formatter, "{err}"),
            Self::InvalidRootHashByteLength { actual } => {
                write!(
                    formatter,
                    "blob root hash byte input must be {BLOB_ROOT_HASH_BYTE_LENGTH} bytes, got {actual}"
                )
            }
        }
    }
}

impl Error for BlobStorageConversionError {}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_gateway_bytes_to_toko_root_hash_text() {
        let bytes = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0xf0, 0xe0, 0xd0, 0xc0, 0xb0, 0xa0, 0x90, 0x80, 0x70, 0x60, 0x50, 0x40,
            0x30, 0x20, 0x10, 0xff,
        ];

        let hash = BlobStorageConversionOps::root_hash_from_bytes(&bytes).expect("bytes convert");

        assert_eq!(
            hash.as_str(),
            "sha256:000102030405060708090a0b0c0d0e0ff0e0d0c0b0a0908070605040302010ff"
        );
    }

    #[test]
    fn rejects_gateway_byte_inputs_that_are_not_32_bytes() {
        assert_eq!(
            BlobStorageConversionOps::root_hash_from_bytes(&[0u8; 31]),
            Err(BlobStorageConversionError::InvalidRootHashByteLength { actual: 31 })
        );
    }

    #[test]
    fn parses_text_hash_through_model_validator() {
        let hash = BlobStorageConversionOps::root_hash_from_text(
            "sha256:FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        )
        .expect("hash parses");

        assert_eq!(
            hash.as_str(),
            "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        );
    }

    #[test]
    fn rejects_text_hash_with_typed_model_error() {
        assert_eq!(
            BlobStorageConversionOps::root_hash_from_text("sha256:zz"),
            Err(BlobStorageConversionError::InvalidRootHash(
                BlobRootHashError::InvalidLength { actual: 9 }
            ))
        );
    }
}
