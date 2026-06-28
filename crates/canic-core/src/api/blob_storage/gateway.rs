//! Module: api::blob_storage::gateway
//!
//! Responsibility: expose gateway-principal and gateway-scoped deletion helpers.
//! Does not own: stable storage schema, billing configuration, or endpoint auth.
//! Boundary: delegates gateway membership checks and state mutation to ops.

use super::BlobStorageApi;
use crate::{
    cdk::types::Principal,
    ops::blob_storage::{conversion::BlobStorageConversionOps, lifecycle::BlobStorageLifecycleOps},
};

impl BlobStorageApi {
    /// Return pending-deletion roots only to registered storage gateways.
    #[must_use]
    pub fn pending_deletion_hashes_for_gateway(caller: Principal) -> Vec<String> {
        if !BlobStorageLifecycleOps::is_gateway_principal(caller) {
            return Vec::new();
        }
        BlobStorageLifecycleOps::pending_deletion_hashes()
    }

    /// Confirm gateway deletion for valid 32-byte roots when caller is a registered gateway.
    pub fn confirm_deleted_by_gateway_hash_bytes_batch(
        caller: Principal,
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
    pub fn upsert_gateway_principal(principal: Principal, now_ns: u64) {
        BlobStorageLifecycleOps::upsert_gateway_principal(principal, now_ns);
    }

    /// Replace authorized storage gateway principals.
    #[must_use]
    pub fn replace_gateway_principals(principals: &[Principal], now_ns: u64) -> u64 {
        BlobStorageLifecycleOps::replace_gateway_principals(principals, now_ns)
    }

    /// Remove an authorized storage gateway principal.
    #[must_use]
    pub fn remove_gateway_principal(principal: Principal) -> bool {
        BlobStorageLifecycleOps::remove_gateway_principal(principal)
    }

    /// Return the number of authorized storage gateway principals.
    #[must_use]
    pub fn gateway_principal_count() -> u64 {
        BlobStorageLifecycleOps::gateway_principal_count()
    }

    /// Return whether the principal is an authorized storage gateway.
    #[must_use]
    pub fn is_gateway_principal(principal: Principal) -> bool {
        BlobStorageLifecycleOps::is_gateway_principal(principal)
    }
}
