use crate::dto::prelude::*;

#[cfg(feature = "blob-storage-billing")]
pub use crate::domain::blob_storage::{
    BlobStorageBillingWarning, BlobStorageFundingStatus, BlobStorageGatewayPrincipalSyncAction,
    BlobStoragePaymentModelStatus, BlobStorageReadinessBlocker,
};
#[cfg(feature = "blob-storage-billing")]
use candid::Int;

///
/// CreateCertificateResult
///
/// Passive DTO returned by the blob-storage create-certificate endpoint.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CreateCertificateResult {
    pub method: String,
    pub blob_hash: String,
}

///
/// BlobStorageLocalCounters
///
/// Passive DTO for host-owned blob-storage status wrappers.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageLocalCounters {
    pub stored_blobs: u64,
    pub pending_deletions: u64,
    pub gateway_principals: u64,
}

impl BlobStorageLocalCounters {
    #[must_use]
    pub const fn new(stored_blobs: u64, pending_deletions: u64, gateway_principals: u64) -> Self {
        Self {
            stored_blobs,
            pending_deletions,
            gateway_principals,
        }
    }
}

///
/// BlobStorageCashierDebtTarget
///
/// Passive DTO for the Cashier account balance debt-target variant.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStorageCashierDebtTarget {
    Prepaid,
    Ledger,
}

///
/// BlobStorageCashierAccountCycleBalances
///
/// Passive DTO for Cashier cycle-balance records.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageCashierAccountCycleBalances {
    pub total: Int,
    pub cycles_prepaid: Int,
    pub cycles_promo: Int,
    pub debt_target: BlobStorageCashierDebtTarget,
    pub cycles_ledger: Int,
}

///
/// BlobStorageCashierAccountBalanceGetRequest
///
/// Passive DTO for `account_balance_get_v1` requests.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageCashierAccountBalanceGetRequest {
    pub account: Principal,
}

///
/// BlobStorageCashierAccountBalanceGetOk
///
/// Passive DTO for successful `account_balance_get_v1` responses.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageCashierAccountBalanceGetOk {
    pub account_cycle_balances: BlobStorageCashierAccountCycleBalances,
    pub account: Principal,
}

///
/// BlobStorageCashierAccountBalanceGetError
///
/// Passive DTO for Cashier `account_balance_get_v1` error variants.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStorageCashierAccountBalanceGetError {
    AccountNotFound,
    InternalError(String),
}

///
/// BlobStorageCashierAccountBalanceGetResult
///
/// Passive DTO for Cashier `account_balance_get_v1` results.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStorageCashierAccountBalanceGetResult {
    Ok(BlobStorageCashierAccountBalanceGetOk),
    Err(BlobStorageCashierAccountBalanceGetError),
}

///
/// BlobStorageCashierAccountTopUpRequest
///
/// Passive DTO for `account_top_up_v1` request records.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageCashierAccountTopUpRequest {
    pub target_balance: Option<Nat>,
    pub account: Option<Principal>,
}

///
/// BlobStorageCashierAccountTopUpOk
///
/// Passive DTO for successful `account_top_up_v1` responses.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageCashierAccountTopUpOk {
    pub balance: BlobStorageCashierAccountCycleBalances,
    pub message: String,
}

///
/// BlobStorageCashierAccountTopUpError
///
/// Passive DTO for Cashier `account_top_up_v1` error variants.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStorageCashierAccountTopUpError {
    NotAuthorized(Principal),
    AccountBalanceOverflow,
    InternalError(String),
    TopUpWithoutCycles,
}

///
/// BlobStorageCashierAccountTopUpResult
///
/// Passive DTO for Cashier `account_top_up_v1` results.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStorageCashierAccountTopUpResult {
    Ok(BlobStorageCashierAccountTopUpOk),
    Err(BlobStorageCashierAccountTopUpError),
}

///
/// BlobStorageBillingConfig
///
/// Passive DTO for internal blob-storage billing configuration.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageBillingConfig {
    pub cashier_canister_id: Principal,
    pub project_cycles_reserve: Nat,
    pub min_upload_balance: Nat,
    pub target_upload_balance: Nat,
    pub gateway_principal_limit: u64,
}

///
/// BlobProjectCyclesTopUpReport
///
/// Passive DTO returned by `_immutableObjectStorageFundFromProjectCycles`.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobProjectCyclesTopUpReport {
    pub requested_cycles: Nat,
    pub attached_cycles: Nat,
    pub project_cycles_before: Nat,
    pub project_cycles_after: Nat,
    pub reserve_cycles: Nat,
    pub cashier_total_after: Nat,
    pub skipped_reason: Option<String>,
}

///
/// BlobStorageStatusRequest
///
/// Passive DTO for backend blob-storage billing status requests.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageStatusRequest {
    pub sync_gateway_principals: bool,
}

///
/// BlobStorageStatusResponse
///
/// Passive DTO returned by `get_blob_storage_status`.
///

#[cfg(feature = "blob-storage-billing")]
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageStatusResponse {
    pub payment_model: BlobStoragePaymentModelStatus,
    pub cashier_canister_id: Option<Principal>,
    pub payment_account: Option<Principal>,
    pub cashier_balance: Option<Nat>,
    pub min_upload_balance: Option<Nat>,
    pub target_upload_balance: Option<Nat>,
    pub project_cycles_reserve: Option<Nat>,
    pub project_cycles_available: Nat,
    pub gateway_principal_count: u64,
    pub last_gateway_principal_sync_at_ns: Option<u64>,
    pub gateway_principal_sync_action: BlobStorageGatewayPrincipalSyncAction,
    pub funding_status: BlobStorageFundingStatus,
    pub ready: bool,
    pub blockers: Vec<BlobStorageReadinessBlocker>,
    pub warnings: Vec<BlobStorageBillingWarning>,
}

#[cfg(all(test, feature = "blob-storage-billing"))]
mod tests {
    use super::*;
    use candid::{CandidType, Decode, Encode};
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[test]
    fn billing_status_enums_roundtrip_candid_through_dto_path() {
        assert_enum_candid_contract(BlobStoragePaymentModelStatus::ProjectAsPaymentAccount);
        assert_enum_candid_contract(BlobStorageGatewayPrincipalSyncAction::SkippedReadOnlyStatus);
        assert_enum_candid_contract(BlobStorageFundingStatus::FundingRequired {
            requested_cycles: Nat::from(42_u64),
        });
        assert_enum_candid_contract(BlobStorageReadinessBlocker::InsufficientCashierBalance);
        assert_enum_candid_contract(BlobStorageBillingWarning::SyncRequestedButStatusIsReadOnly);
    }

    fn assert_enum_candid_contract<T>(value: T)
    where
        T: CandidType + Clone + Debug + DeserializeOwned + Eq,
    {
        let bytes = Encode!(&value).expect("encode blob-storage status enum");
        let decoded = Decode!(&bytes, T).expect("decode blob-storage status enum");

        assert_eq!(decoded, value);
    }
}
