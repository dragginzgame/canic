//! Module: ops::blob_storage::lifecycle
//!
//! Responsibility: enforce blob-storage lifecycle invariants over stable records.
//! Does not own: endpoint guards, async gateway calls, or external principal synchronization.
//! Boundary: workflow passes admitted inputs here before stable mutation.

use crate::{
    cdk::types::Principal,
    model::blob_storage::BlobRootHash,
    storage::stable::blob_storage::{
        BlobDeletionPendingRecord, BlobStorageStore, StorageGatewayPrincipalRecord,
        StoredBlobRecord,
    },
};
use thiserror::Error as ThisError;

#[cfg(feature = "blob-storage-billing")]
use crate::{
    cdk::candid::Nat, dto::blob_storage::BlobStorageBillingConfig,
    storage::stable::blob_storage::BlobStorageBillingConfigRecord,
    view::blob_storage::BlobStorageBillingConfigView,
};

///
/// BlobStorageLifecycleOps
///
/// Zero-cost namespace for non-billing blob-storage lifecycle operations.
///

pub struct BlobStorageLifecycleOps;

impl BlobStorageLifecycleOps {
    #[cfg(feature = "blob-storage-billing")]
    #[must_use]
    pub fn billing_config() -> Option<BlobStorageBillingConfigView> {
        BlobStorageStore::billing_config().map(Self::billing_config_record_to_view)
    }

    #[cfg(feature = "blob-storage-billing")]
    #[must_use]
    pub fn billing_config_dto() -> Option<BlobStorageBillingConfig> {
        Self::billing_config().map(Self::billing_config_view_to_dto)
    }

    #[cfg(feature = "blob-storage-billing")]
    pub fn set_billing_config(
        cashier_canister_id: Principal,
        project_cycles_reserve: u128,
        min_upload_balance: u128,
        target_upload_balance: u128,
        gateway_principal_limit: u64,
        updated_at_ns: u64,
    ) {
        BlobStorageStore::set_billing_config(BlobStorageBillingConfigRecord::new(
            cashier_canister_id,
            project_cycles_reserve,
            min_upload_balance,
            target_upload_balance,
            gateway_principal_limit,
            updated_at_ns,
        ));
    }

    #[cfg(feature = "blob-storage-billing")]
    pub fn record_gateway_principal_sync(now_ns: u64) {
        BlobStorageStore::set_last_gateway_principal_sync_at_ns(now_ns);
    }

    #[cfg(feature = "blob-storage-billing")]
    #[must_use]
    pub fn last_gateway_principal_sync_at_ns() -> Option<u64> {
        BlobStorageStore::last_gateway_principal_sync_at_ns()
    }

    /// Register a live blob, returning whether this call inserted new live state.
    pub fn register_live(
        hash: &BlobRootHash,
        now_ns: u64,
    ) -> Result<BlobRegisterOutcome, BlobStorageLifecycleError> {
        if BlobStorageStore::get_pending_deletion(hash).is_some() {
            return Err(BlobStorageLifecycleError::BlobPendingDeletion);
        }

        if BlobStorageStore::get_stored_blob(hash).is_some() {
            return Ok(BlobRegisterOutcome::AlreadyLive);
        }

        BlobStorageStore::upsert_stored_blob(hash, StoredBlobRecord::new(hash, now_ns));
        Ok(BlobRegisterOutcome::Registered)
    }

    /// Return whether the blob is registered and not pending deletion.
    #[must_use]
    pub fn is_live(hash: &BlobRootHash) -> bool {
        BlobStorageStore::get_stored_blob(hash).is_some()
            && BlobStorageStore::get_pending_deletion(hash).is_none()
    }

    /// Require a live blob and return its stored record.
    pub fn require_live(
        hash: &BlobRootHash,
    ) -> Result<StoredBlobRecord, BlobStorageLifecycleError> {
        let Some(record) = BlobStorageStore::get_stored_blob(hash) else {
            return Err(BlobStorageLifecycleError::BlobNotLive);
        };
        if BlobStorageStore::get_pending_deletion(hash).is_some() {
            return Err(BlobStorageLifecycleError::BlobPendingDeletion);
        }
        Ok(record)
    }

    /// Mark a live blob as pending gateway deletion.
    pub fn mark_pending_delete(
        hash: &BlobRootHash,
        now_ns: u64,
    ) -> Result<BlobPendingDeletionOutcome, BlobStorageLifecycleError> {
        if BlobStorageStore::get_stored_blob(hash).is_none() {
            return Err(BlobStorageLifecycleError::BlobNotLive);
        }
        if BlobStorageStore::get_pending_deletion(hash).is_some() {
            return Ok(BlobPendingDeletionOutcome::AlreadyPendingDeletion);
        }

        BlobStorageStore::upsert_pending_deletion(
            hash,
            BlobDeletionPendingRecord::new(hash, now_ns),
        );
        Ok(BlobPendingDeletionOutcome::MarkedPendingDeletion)
    }

    /// Confirm gateway deletion. Absent inputs are no-ops, matching Toko endpoint behavior.
    pub fn confirm_deleted_by_gateway(hash: &BlobRootHash) {
        BlobStorageStore::remove_pending_deletion(hash);
        BlobStorageStore::remove_stored_blob(hash);
    }

    /// Return the number of stored blob records, including pending-deletion records.
    #[must_use]
    pub fn stored_blob_count() -> u64 {
        BlobStorageStore::stored_blob_count()
    }

    /// Return the number of pending gateway-deletion records.
    #[must_use]
    pub fn pending_deletion_count() -> u64 {
        BlobStorageStore::pending_deletion_count()
    }

    /// Return pending-deletion root hashes in stable key order.
    #[must_use]
    pub fn pending_deletion_hashes() -> Vec<String> {
        BlobStorageStore::pending_deletions_data()
            .entries
            .into_iter()
            .map(|entry| entry.key.as_str().to_string())
            .collect()
    }

    /// Insert or update an authorized storage gateway principal.
    pub fn upsert_gateway_principal(principal: Principal, now_ns: u64) {
        BlobStorageStore::upsert_gateway_principal(
            principal,
            StorageGatewayPrincipalRecord::new(principal, now_ns),
        );
    }

    /// Replace the authorized storage gateway principal set.
    #[must_use]
    pub fn replace_gateway_principals(principals: &[Principal], now_ns: u64) -> u64 {
        for record in BlobStorageStore::gateway_principals_data().entries {
            if !principals.contains(&record.gateway_principal) {
                BlobStorageStore::remove_gateway_principal(record.gateway_principal);
            }
        }

        for principal in principals {
            BlobStorageStore::upsert_gateway_principal(
                *principal,
                StorageGatewayPrincipalRecord::new(*principal, now_ns),
            );
        }

        BlobStorageStore::gateway_principal_count()
    }

    /// Remove an authorized storage gateway principal.
    pub fn remove_gateway_principal(principal: Principal) -> bool {
        BlobStorageStore::remove_gateway_principal(principal).is_some()
    }

    /// Return the number of authorized storage gateway principals.
    #[must_use]
    pub fn gateway_principal_count() -> u64 {
        BlobStorageStore::gateway_principal_count()
    }

    /// Return whether the principal is an authorized storage gateway.
    #[must_use]
    pub fn is_gateway_principal(principal: Principal) -> bool {
        BlobStorageStore::get_gateway_principal(principal).is_some()
    }

    #[cfg(feature = "blob-storage-billing")]
    const fn billing_config_record_to_view(
        record: BlobStorageBillingConfigRecord,
    ) -> BlobStorageBillingConfigView {
        BlobStorageBillingConfigView::new(
            record.cashier_canister_id,
            record.project_cycles_reserve,
            record.min_upload_balance,
            record.target_upload_balance,
            record.gateway_principal_limit,
            record.updated_at_ns,
        )
    }

    #[cfg(feature = "blob-storage-billing")]
    fn billing_config_view_to_dto(view: BlobStorageBillingConfigView) -> BlobStorageBillingConfig {
        BlobStorageBillingConfig {
            cashier_canister_id: view.cashier_canister_id,
            project_cycles_reserve: Self::nat_from_u128(view.project_cycles_reserve),
            min_upload_balance: Self::nat_from_u128(view.min_upload_balance),
            target_upload_balance: Self::nat_from_u128(view.target_upload_balance),
            gateway_principal_limit: view.gateway_principal_limit,
        }
    }

    #[cfg(feature = "blob-storage-billing")]
    fn nat_from_u128(value: u128) -> Nat {
        Nat::parse(value.to_string().as_bytes()).expect("u128 must encode as Candid nat")
    }
}

///
/// BlobRegisterOutcome
///
/// Result of idempotent live-blob registration.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlobRegisterOutcome {
    Registered,
    AlreadyLive,
}

impl BlobRegisterOutcome {
    #[must_use]
    pub const fn inserted(self) -> bool {
        matches!(self, Self::Registered)
    }
}

///
/// BlobPendingDeletionOutcome
///
/// Result of idempotent pending-deletion marking.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlobPendingDeletionOutcome {
    MarkedPendingDeletion,
    AlreadyPendingDeletion,
}

impl BlobPendingDeletionOutcome {
    #[must_use]
    pub const fn inserted(self) -> bool {
        matches!(self, Self::MarkedPendingDeletion)
    }
}

///
/// BlobStorageLifecycleError
///
/// Typed lifecycle failure for non-billing blob-storage operations.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum BlobStorageLifecycleError {
    #[error("blob is not registered live")]
    BlobNotLive,

    #[error("blob is pending deletion")]
    BlobPendingDeletion,
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::blob_storage::BlobRootHash, storage::stable::blob_storage::BlobStorageStore,
    };

    fn hash(value: &str) -> BlobRootHash {
        BlobRootHash::try_from(value).expect("valid blob root hash")
    }

    fn h1() -> BlobRootHash {
        hash("sha256:1111111111111111111111111111111111111111111111111111111111111111")
    }

    fn h2() -> BlobRootHash {
        hash("sha256:2222222222222222222222222222222222222222222222222222222222222222")
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn register_live_is_idempotent_until_pending_deletion() {
        BlobStorageStore::clear();
        let hash = h1();

        assert_eq!(
            BlobStorageLifecycleOps::register_live(&hash, 10).expect("register"),
            BlobRegisterOutcome::Registered
        );
        assert_eq!(
            BlobStorageLifecycleOps::register_live(&hash, 20).expect("register again"),
            BlobRegisterOutcome::AlreadyLive
        );
        assert!(BlobStorageLifecycleOps::is_live(&hash));

        BlobStorageLifecycleOps::mark_pending_delete(&hash, 30).expect("mark pending");

        assert_eq!(
            BlobStorageLifecycleOps::register_live(&hash, 40),
            Err(BlobStorageLifecycleError::BlobPendingDeletion)
        );
        assert!(!BlobStorageLifecycleOps::is_live(&hash));
    }

    #[test]
    fn mark_pending_delete_requires_live_blob() {
        BlobStorageStore::clear();
        let hash = h1();

        assert_eq!(
            BlobStorageLifecycleOps::mark_pending_delete(&hash, 10),
            Err(BlobStorageLifecycleError::BlobNotLive)
        );
    }

    #[test]
    fn gateway_confirmation_removes_pending_and_live_state() {
        BlobStorageStore::clear();
        let hash = h1();

        BlobStorageLifecycleOps::register_live(&hash, 10).expect("register");
        assert_eq!(BlobStorageLifecycleOps::stored_blob_count(), 1);
        assert_eq!(BlobStorageLifecycleOps::pending_deletion_count(), 0);

        BlobStorageLifecycleOps::mark_pending_delete(&hash, 20).expect("mark pending");
        assert_eq!(BlobStorageLifecycleOps::stored_blob_count(), 1);
        assert_eq!(BlobStorageLifecycleOps::pending_deletion_count(), 1);
        assert_eq!(
            BlobStorageLifecycleOps::pending_deletion_hashes(),
            vec![hash.as_str().to_string()]
        );

        BlobStorageLifecycleOps::confirm_deleted_by_gateway(&hash);

        assert!(!BlobStorageLifecycleOps::is_live(&hash));
        assert_eq!(BlobStorageLifecycleOps::stored_blob_count(), 0);
        assert_eq!(BlobStorageLifecycleOps::pending_deletion_count(), 0);
        assert!(BlobStorageLifecycleOps::pending_deletion_hashes().is_empty());
        BlobStorageLifecycleOps::confirm_deleted_by_gateway(&hash);
        assert_eq!(BlobStorageLifecycleOps::stored_blob_count(), 0);
        assert_eq!(BlobStorageLifecycleOps::pending_deletion_count(), 0);
    }

    #[test]
    fn gateway_confirmation_matches_inventory_edge_cases() {
        BlobStorageStore::clear();
        let unknown = h1();
        let live_only = h2();

        BlobStorageLifecycleOps::confirm_deleted_by_gateway(&unknown);
        assert_eq!(BlobStorageLifecycleOps::stored_blob_count(), 0);
        assert_eq!(BlobStorageLifecycleOps::pending_deletion_count(), 0);

        BlobStorageLifecycleOps::register_live(&live_only, 10).expect("register");
        assert!(BlobStorageLifecycleOps::is_live(&live_only));

        BlobStorageLifecycleOps::confirm_deleted_by_gateway(&live_only);

        assert!(!BlobStorageLifecycleOps::is_live(&live_only));
        assert_eq!(BlobStorageLifecycleOps::stored_blob_count(), 0);
        assert_eq!(BlobStorageLifecycleOps::pending_deletion_count(), 0);
    }

    #[test]
    fn re_registration_after_confirmation_requires_explicit_register() {
        BlobStorageStore::clear();
        let hash = h1();

        BlobStorageLifecycleOps::register_live(&hash, 10).expect("register");
        BlobStorageLifecycleOps::mark_pending_delete(&hash, 20).expect("mark pending");
        BlobStorageLifecycleOps::confirm_deleted_by_gateway(&hash);

        assert!(!BlobStorageLifecycleOps::is_live(&hash));
        assert_eq!(BlobStorageLifecycleOps::stored_blob_count(), 0);
        assert_eq!(BlobStorageLifecycleOps::pending_deletion_count(), 0);

        assert_eq!(
            BlobStorageLifecycleOps::register_live(&hash, 30).expect("explicit re-register"),
            BlobRegisterOutcome::Registered
        );
        assert!(BlobStorageLifecycleOps::is_live(&hash));
        assert_eq!(BlobStorageLifecycleOps::stored_blob_count(), 1);
        assert_eq!(BlobStorageLifecycleOps::pending_deletion_count(), 0);
    }

    #[test]
    fn gateway_principal_registry_is_idempotent() {
        BlobStorageStore::clear();
        let gateway = p(42);

        assert!(!BlobStorageLifecycleOps::is_gateway_principal(gateway));
        assert_eq!(BlobStorageLifecycleOps::gateway_principal_count(), 0);
        BlobStorageLifecycleOps::upsert_gateway_principal(gateway, 10);
        BlobStorageLifecycleOps::upsert_gateway_principal(gateway, 20);
        assert!(BlobStorageLifecycleOps::is_gateway_principal(gateway));
        assert_eq!(BlobStorageLifecycleOps::gateway_principal_count(), 1);

        assert!(BlobStorageLifecycleOps::remove_gateway_principal(gateway));
        assert!(!BlobStorageLifecycleOps::remove_gateway_principal(gateway));
        assert!(!BlobStorageLifecycleOps::is_gateway_principal(gateway));
        assert_eq!(BlobStorageLifecycleOps::gateway_principal_count(), 0);
    }

    #[test]
    fn replacing_gateway_principals_removes_absent_and_keeps_present_members() {
        BlobStorageStore::clear();
        let first = p(1);
        let second = p(2);
        let third = p(3);

        BlobStorageLifecycleOps::upsert_gateway_principal(first, 10);
        BlobStorageLifecycleOps::upsert_gateway_principal(second, 20);

        assert_eq!(
            BlobStorageLifecycleOps::replace_gateway_principals(&[second, third], 30),
            2
        );

        assert!(!BlobStorageLifecycleOps::is_gateway_principal(first));
        assert!(BlobStorageLifecycleOps::is_gateway_principal(second));
        assert!(BlobStorageLifecycleOps::is_gateway_principal(third));
    }
}
