//! Module: api::blob_storage
//!
//! Responsibility: expose blob-storage helpers used by macro-generated endpoints.
//! Does not own: stable storage, gateway authorization, or lifecycle workflows.
//! Boundary: delegates to workflow and maps typed failures into public errors.

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
    fn map_conversion_error(_err: BlobStorageConversionError) -> Error {
        Error::from_registered(crate::diagnostics::codes::REQUEST_INVALID)
    }

    fn map_lifecycle_error(err: BlobStorageLifecycleError) -> Error {
        match err {
            BlobStorageLifecycleError::BlobNotLive => {
                Error::from_registered(crate::diagnostics::codes::COLLECTION_UNAVAILABLE)
            }
            BlobStorageLifecycleError::BlobPendingDeletion => {
                Error::from_registered(crate::diagnostics::codes::STATE_CONFLICT)
            }
        }
    }
}
