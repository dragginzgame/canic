//! Module: api::blob_storage
//!
//! Responsibility: expose blob-storage helpers used by macro-generated endpoints.
//! Does not own: stable storage, gateway authorization, or lifecycle workflows.
//! Boundary: maps public endpoint inputs into ops validation and public errors.

#[cfg(feature = "blob-storage-billing")]
mod billing;
mod gateway;
mod hash;
mod lifecycle;
#[cfg(test)]
mod tests;

use crate::{
    dto::error::Error,
    ops::blob_storage::{
        conversion::BlobStorageConversionError, lifecycle::BlobStorageLifecycleError,
    },
};

///
/// BlobStorageApi
///
/// Public facade for feature-gated blob-storage endpoint helpers.
///

pub struct BlobStorageApi;

impl BlobStorageApi {
    fn map_conversion_error(err: BlobStorageConversionError) -> Error {
        Error::invalid(err.to_string())
    }

    fn map_lifecycle_error(err: BlobStorageLifecycleError) -> Error {
        match err {
            BlobStorageLifecycleError::BlobNotLive => Error::not_found(err.to_string()),
            BlobStorageLifecycleError::BlobPendingDeletion => Error::conflict(err.to_string()),
        }
    }
}
