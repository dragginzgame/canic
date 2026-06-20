//! Module: api::blob_storage
//!
//! Responsibility: expose blob-storage helpers used by macro-generated endpoints.
//! Does not own: stable storage, gateway authorization, or lifecycle workflows.
//! Boundary: maps public endpoint inputs into ops/model validation and public errors.

use crate::{
    dto::{blob_storage::CreateCertificateResult, error::Error},
    ops::{
        blob_storage::{
            conversion::{BlobStorageConversionError, BlobStorageConversionOps},
            lifecycle::{
                BlobPendingDeletionOutcome, BlobRegisterOutcome, BlobStorageLifecycleError,
                BlobStorageLifecycleOps,
            },
        },
        ic::IcOps,
    },
};

///
/// BlobStorageApi
///
/// Public facade for feature-gated blob-storage endpoint helpers.
///

pub struct BlobStorageApi;

impl BlobStorageApi {
    /// Canonicalize a Toko/Caffeine root hash string into `sha256:<64-lowercase-hex>`.
    pub fn canonical_root_hash_text(value: &str) -> Result<String, Error> {
        BlobStorageConversionOps::root_hash_from_text(value)
            .map(crate::model::blob_storage::BlobRootHash::into_string)
            .map_err(Self::map_conversion_error)
    }

    /// Canonicalize a gateway 32-byte root hash into `sha256:<64-lowercase-hex>`.
    pub fn canonical_root_hash_bytes(bytes: &[u8]) -> Result<String, Error> {
        BlobStorageConversionOps::root_hash_from_bytes(bytes)
            .map(crate::model::blob_storage::BlobRootHash::into_string)
            .map_err(Self::map_conversion_error)
    }

    /// Register a live blob root. Returns `true` when new live state was inserted.
    pub fn register_live(root_hash: &str, now_ns: u64) -> Result<bool, Error> {
        let hash = BlobStorageConversionOps::root_hash_from_text(root_hash)
            .map_err(Self::map_conversion_error)?;
        BlobStorageLifecycleOps::register_live(&hash, now_ns)
            .map(BlobRegisterOutcome::inserted)
            .map_err(Self::map_lifecycle_error)
    }

    /// Register an upload certificate request and return the gateway-compatible DTO.
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

    /// Return pending-deletion root hashes in stable key order.
    #[must_use]
    pub fn pending_deletion_hashes() -> Vec<String> {
        BlobStorageLifecycleOps::pending_deletion_hashes()
    }

    /// Return pending-deletion roots only to registered storage gateways.
    #[must_use]
    pub fn pending_deletion_hashes_for_gateway(
        caller: crate::cdk::types::Principal,
    ) -> Vec<String> {
        if !BlobStorageLifecycleOps::is_gateway_principal(caller) {
            return Vec::new();
        }
        BlobStorageLifecycleOps::pending_deletion_hashes()
    }

    /// Confirm gateway deletion for valid 32-byte roots when caller is a registered gateway.
    pub fn confirm_deleted_by_gateway_hash_bytes_batch(
        caller: crate::cdk::types::Principal,
        hash_bytes_list: Vec<Vec<u8>>,
    ) {
        if !BlobStorageLifecycleOps::is_gateway_principal(caller) {
            return;
        }

        for bytes in &hash_bytes_list {
            if let Ok(hash) = BlobStorageConversionOps::root_hash_from_bytes(bytes) {
                BlobStorageLifecycleOps::confirm_deleted_by_gateway(&hash);
            }
        }
    }

    /// Insert or update an authorized storage gateway principal.
    pub fn upsert_gateway_principal(principal: crate::cdk::types::Principal, now_ns: u64) {
        BlobStorageLifecycleOps::upsert_gateway_principal(principal, now_ns);
    }

    /// Remove an authorized storage gateway principal.
    #[must_use]
    pub fn remove_gateway_principal(principal: crate::cdk::types::Principal) -> bool {
        BlobStorageLifecycleOps::remove_gateway_principal(principal)
    }

    /// Return the number of authorized storage gateway principals.
    #[must_use]
    pub fn gateway_principal_count() -> u64 {
        BlobStorageLifecycleOps::gateway_principal_count()
    }

    /// Return whether the principal is an authorized storage gateway.
    #[must_use]
    pub fn is_gateway_principal(principal: crate::cdk::types::Principal) -> bool {
        BlobStorageLifecycleOps::is_gateway_principal(principal)
    }

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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::error::ErrorCode;

    #[test]
    fn canonical_root_hash_text_normalizes_toko_hashes() {
        let hash = BlobStorageApi::canonical_root_hash_text(
            "sha256:ABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD",
        )
        .expect("hash parses");

        assert_eq!(
            hash,
            "sha256:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        );
    }

    #[test]
    fn canonical_root_hash_bytes_matches_gateway_query_shape() {
        let hash =
            BlobStorageApi::canonical_root_hash_bytes(&[0xabu8; 32]).expect("hash bytes convert");

        assert_eq!(
            hash,
            "sha256:abababababababababababababababababababababababababababababababab"
        );
    }

    #[test]
    fn malformed_root_hash_maps_to_public_invalid_input() {
        let err = BlobStorageApi::canonical_root_hash_text("sha256:zz")
            .expect_err("short malformed hash should fail");

        assert_eq!(err.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn live_blob_lifecycle_maps_to_public_api() {
        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        let hash = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

        assert!(!BlobStorageApi::is_live(hash).expect("live check"));
        assert_eq!(BlobStorageApi::stored_blob_count(), 0);
        assert_eq!(BlobStorageApi::pending_deletion_count(), 0);
        assert!(BlobStorageApi::register_live(hash, 10).expect("register"));
        assert!(!BlobStorageApi::register_live(hash, 20).expect("register again"));
        assert!(BlobStorageApi::is_live(hash).expect("live check"));
        assert_eq!(BlobStorageApi::stored_blob_count(), 1);
        assert_eq!(BlobStorageApi::pending_deletion_count(), 0);
        BlobStorageApi::require_live(hash).expect("require live");

        assert!(BlobStorageApi::mark_pending_delete(hash, 30).expect("mark pending"));
        assert_eq!(BlobStorageApi::stored_blob_count(), 1);
        assert_eq!(BlobStorageApi::pending_deletion_count(), 1);
        assert_eq!(
            BlobStorageApi::require_live(hash)
                .expect_err("pending is not live")
                .code,
            ErrorCode::Conflict
        );
    }

    #[test]
    fn gateway_byte_confirmation_removes_live_blob() {
        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        let hash = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
        let bytes = [0xddu8; 32];

        BlobStorageApi::register_live(hash, 10).expect("register");
        BlobStorageApi::mark_pending_delete(hash, 20).expect("mark pending");
        assert_eq!(
            BlobStorageApi::pending_deletion_hashes(),
            vec![hash.to_string()]
        );

        BlobStorageApi::confirm_deleted_by_gateway_hash_bytes(&bytes).expect("confirm");

        assert!(!BlobStorageApi::is_live(hash).expect("live check"));
        assert!(BlobStorageApi::pending_deletion_hashes().is_empty());
    }

    #[test]
    fn gateway_principal_api_is_idempotent() {
        let principal = crate::cdk::types::Principal::from_slice(&[99; 29]);

        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        assert!(!BlobStorageApi::is_gateway_principal(principal));
        assert_eq!(BlobStorageApi::gateway_principal_count(), 0);

        BlobStorageApi::upsert_gateway_principal(principal, 10);
        assert!(BlobStorageApi::is_gateway_principal(principal));
        assert_eq!(BlobStorageApi::gateway_principal_count(), 1);
        assert!(BlobStorageApi::remove_gateway_principal(principal));
        assert!(!BlobStorageApi::remove_gateway_principal(principal));
        assert_eq!(BlobStorageApi::gateway_principal_count(), 0);
    }

    #[test]
    fn gateway_endpoint_helpers_match_toko_malformed_input_behavior() {
        crate::storage::stable::blob_storage::BlobStorageStore::clear();
        let hash = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        let bytes = [0xeeu8; 32];
        let gateway = crate::cdk::types::Principal::from_slice(&[11; 29]);

        assert_eq!(
            BlobStorageApi::blobs_are_live(vec![bytes.to_vec(), vec![1, 2, 3]]),
            vec![false, false]
        );

        BlobStorageApi::create_certificate(hash.to_string()).expect("create certificate");
        assert_eq!(
            BlobStorageApi::blobs_are_live(vec![bytes.to_vec(), vec![1, 2, 3]]),
            vec![true, false]
        );

        BlobStorageApi::mark_pending_delete(hash, 10).expect("mark pending");
        assert!(BlobStorageApi::pending_deletion_hashes_for_gateway(gateway).is_empty());

        BlobStorageApi::upsert_gateway_principal(gateway, 20);
        assert_eq!(
            BlobStorageApi::pending_deletion_hashes_for_gateway(gateway),
            vec![hash.to_string()]
        );

        BlobStorageApi::confirm_deleted_by_gateway_hash_bytes_batch(
            gateway,
            vec![vec![1, 2, 3], bytes.to_vec()],
        );

        assert!(BlobStorageApi::pending_deletion_hashes().is_empty());
        assert!(!BlobStorageApi::is_live(hash).expect("live check"));
    }
}
