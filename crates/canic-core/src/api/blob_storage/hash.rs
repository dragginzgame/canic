//! Module: api::blob_storage::hash
//!
//! Responsibility: expose public blob root hash canonicalization helpers.
//! Does not own: model validation or lifecycle storage mutation.
//! Boundary: delegates boundary hash conversion to ops and maps public errors.

use super::BlobStorageApi;
use crate::{dto::error::Error, ops::blob_storage::conversion::BlobStorageConversionOps};

impl BlobStorageApi {
    /// Canonicalize a Toko/Caffeine root hash string into `sha256:<64-lowercase-hex>`.
    pub fn canonical_root_hash_text(value: &str) -> Result<String, Error> {
        BlobStorageConversionOps::canonical_root_hash_text(value)
            .map_err(Self::map_conversion_error)
    }

    /// Canonicalize a gateway 32-byte root hash into `sha256:<64-lowercase-hex>`.
    pub fn canonical_root_hash_bytes(bytes: &[u8]) -> Result<String, Error> {
        BlobStorageConversionOps::canonical_root_hash_bytes(bytes)
            .map_err(Self::map_conversion_error)
    }
}
