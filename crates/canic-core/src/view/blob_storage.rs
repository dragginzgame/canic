//! Module: view::blob_storage
//!
//! Responsibility: expose internal read-only blob-storage projections.
//! Does not own: stable records, endpoint DTOs, or storage mutation.
//! Boundary: ops map stable records into these views before API/workflow use.

use crate::cdk::types::Principal;

///
/// BlobStorageBillingConfigView
///
/// Internal read-only projection of blob-storage billing configuration.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlobStorageBillingConfigView {
    pub cashier_canister_id: Principal,
    pub project_cycles_reserve: u128,
    pub min_upload_balance: u128,
    pub target_upload_balance: u128,
    pub gateway_principal_limit: u64,
    pub updated_at_ns: u64,
}

#[cfg(feature = "blob-storage-billing")]
impl BlobStorageBillingConfigView {
    #[must_use]
    pub const fn new(
        cashier_canister_id: Principal,
        project_cycles_reserve: u128,
        min_upload_balance: u128,
        target_upload_balance: u128,
        gateway_principal_limit: u64,
        updated_at_ns: u64,
    ) -> Self {
        Self {
            cashier_canister_id,
            project_cycles_reserve,
            min_upload_balance,
            target_upload_balance,
            gateway_principal_limit,
            updated_at_ns,
        }
    }
}
