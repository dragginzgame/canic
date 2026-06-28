//! Module: api::blob_storage::lifecycle
//!
//! Responsibility: expose public blob lifecycle helpers used by generated endpoints.
//! Does not own: stable records, gateway membership, or billing orchestration.
//! Boundary: maps endpoint-shaped inputs into blob-storage lifecycle ops.

use super::BlobStorageApi;
use crate::{
    dto::{
        blob_storage::{BlobStorageLocalCounters, CreateCertificateResult},
        error::Error,
    },
    ops::{
        blob_storage::{
            conversion::BlobStorageConversionOps,
            lifecycle::{BlobPendingDeletionOutcome, BlobRegisterOutcome, BlobStorageLifecycleOps},
        },
        ic::IcOps,
    },
};

impl BlobStorageApi {
    /// Register a live blob root. Returns `true` when new live state was inserted.
    pub fn register_live(root_hash: &str, now_ns: u64) -> Result<bool, Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(root_hash)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::register_live(&hash, now_ns)
            .map(BlobRegisterOutcome::inserted)
            .map_err(Self::map_lifecycle_error)
    }

    /// Register an upload certificate request and return the gateway-compatible DTO.
    ///
    /// The gateway contract echoes the request hash in the response; Canic stores
    /// the canonical normalized hash internally.
    pub fn create_certificate(root_hash: String) -> Result<CreateCertificateResult, Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(&root_hash)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::register_live(&hash, IcOps::now_nanos())
            .map_err(Self::map_lifecycle_error)?;

        Ok(CreateCertificateResult {
            method: "upload".to_string(),
            blob_hash: root_hash,
        })
    }

    /// Evaluate gateway liveness query inputs, returning `false` for malformed byte entries.
    #[must_use]
    pub fn blobs_are_live(hash_bytes_list: Vec<Vec<u8>>) -> Vec<bool> {
        hash_bytes_list
            .iter()
            .map(|bytes| {
                let Ok(hash) = BlobStorageConversionOps::root_hash_from_bytes(bytes) else {
                    return false;
                };
                BlobStorageLifecycleOps::is_live(&hash)
            })
            .collect()
    }

    /// Return whether a blob root is registered live and not pending deletion.
    pub fn is_live(root_hash: &str) -> Result<bool, Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(root_hash)
            .map_err(Self::map_conversion_error)?;
        Ok(BlobStorageLifecycleOps::is_live(&hash))
    }

    /// Require a live blob root, returning `NotFound` when it is missing or pending deletion.
    pub fn require_live(root_hash: &str) -> Result<(), Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(root_hash)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::require_live(&hash)
            .map(|_| ())
            .map_err(Self::map_lifecycle_error)
    }

    /// Mark a live blob as pending gateway deletion.
    pub fn mark_pending_delete(root_hash: &str, now_ns: u64) -> Result<bool, Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(root_hash)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::mark_pending_delete(&hash, now_ns)
            .map(BlobPendingDeletionOutcome::inserted)
            .map_err(Self::map_lifecycle_error)
    }

    /// Confirm gateway deletion from the gateway's 32-byte root hash input.
    pub fn confirm_deleted_by_gateway_hash_bytes(bytes: &[u8]) -> Result<(), Error> {
        let hash = BlobStorageConversionOps::root_hash_from_bytes(bytes)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::confirm_deleted_by_gateway(&hash);
        Ok(())
    }

    /// Return the number of stored blob records, including pending-deletion records.
    #[must_use]
    pub fn stored_blob_count() -> u64 {
        BlobStorageLifecycleOps::stored_blob_count()
    }

    /// Return the number of pending gateway-deletion records.
    #[must_use]
    pub fn pending_deletion_count() -> u64 {
        BlobStorageLifecycleOps::pending_deletion_count()
    }

    /// Return local operational counters for host-owned guarded status endpoints.
    #[must_use]
    pub fn local_counters() -> BlobStorageLocalCounters {
        BlobStorageLocalCounters::new(
            Self::stored_blob_count(),
            Self::pending_deletion_count(),
            Self::gateway_principal_count(),
        )
    }

    /// Return pending-deletion root hashes in stable key order.
    #[must_use]
    pub fn pending_deletion_hashes() -> Vec<String> {
        BlobStorageLifecycleOps::pending_deletion_hashes()
    }
}
