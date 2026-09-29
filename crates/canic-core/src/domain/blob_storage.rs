//! Module: domain::blob_storage
//!
//! Responsibility: define pure blob-storage value enums shared by billing
//! status builders and boundary DTOs.
//! Does not own: Cashier request/result DTOs, blob-storage lifecycle storage,
//! billing workflow orchestration, or gateway sync side effects.
//! Boundary: DTO modules re-export these values to preserve public API paths
//! while status builders import the domain owner directly.

use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

///
/// BlobStoragePaymentModelStatus
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStoragePaymentModelStatus {
    NotConfigured,
    ProjectAsPaymentAccount,
}

///
/// BlobStorageGatewayPrincipalSyncAction
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStorageGatewayPrincipalSyncAction {
    NotRequested,
    SkippedConfigMissing,
    SkippedReadOnlyStatus,
}

///
/// BlobStorageFundingStatus
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStorageFundingStatus {
    NotConfigured,
    NotNeeded,
    FundingRequired {
        requested_cycles: Nat,
    },
    BalanceUnavailable,
    BalanceMalformed,
    ReserveWouldBeViolated {
        requested_cycles: Nat,
        transferable_cycles: Nat,
    },
}

///
/// BlobStorageReadinessBlocker
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStorageReadinessBlocker {
    NotConfigured,
    GatewayPrincipalsMissing,
    CashierBalanceUnavailable,
    CashierBalanceMalformed,
    InsufficientCashierBalance,
    ReserveWouldBeViolated,
}

///
/// BlobStorageBillingWarning
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlobStorageBillingWarning {
    GatewayPrincipalSetEmpty,
    CashierBalanceUnavailable,
    CashierBalanceMalformed,
    SyncRequestedButStatusIsReadOnly,
}
