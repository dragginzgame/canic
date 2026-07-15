//! Module: domain::policy::pure::blob_storage
//!
//! Responsibility: decide blob-storage billing admission, funding, and readiness.
//! Does not own: DTO conversion, storage, Cashier calls, guards, or orchestration.

use crate::domain::{
    blob_storage::{
        BlobStorageBillingWarning, BlobStorageGatewayPrincipalSyncAction,
        BlobStorageReadinessBlocker,
    },
    value::Principal,
};
use thiserror::Error as ThisError;

/// Typed rejection from blob-storage billing policy.
#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum BlobStorageBillingPolicyError {
    #[error("cashier_canister_id must be a concrete canister principal")]
    CashierCanisterRequired,

    #[error("gateway_principal_limit must be greater than zero")]
    GatewayPrincipalLimitZero,

    #[error("project_cycles_reserve must be greater than zero")]
    ProjectCyclesReserveZero,

    #[error("min_upload_balance must be greater than zero")]
    MinUploadBalanceZero,

    #[error("target_upload_balance must be greater than zero")]
    TargetUploadBalanceZero,

    #[error("min_upload_balance must be less than or equal to target_upload_balance")]
    MinUploadBalanceExceedsTarget,

    #[error("requested_cycles must be greater than zero")]
    RequestedCyclesZero,
}

/// Pure funding attachment decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlobStorageFundingAttachmentDecision {
    pub project_cycles_available: u128,
    pub attached_cycles: u128,
    pub skipped_reason: Option<&'static str>,
}

/// Cashier balance observation supplied to readiness policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlobStorageCashierBalanceObservation {
    Available(u128),
    Malformed,
    Unavailable,
}

/// Pure funding-status decision before public Nat projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlobStorageFundingStatusDecision {
    NotConfigured,
    NotNeeded,
    FundingRequired {
        requested_cycles: u128,
    },
    BalanceUnavailable,
    BalanceMalformed,
    ReserveWouldBeViolated {
        requested_cycles: u128,
        transferable_cycles: u128,
    },
}

/// Pure readiness decision for one status response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlobStorageStatusDecision {
    pub gateway_principal_sync_action: BlobStorageGatewayPrincipalSyncAction,
    pub funding_status: BlobStorageFundingStatusDecision,
    pub ready: bool,
    pub blockers: Vec<BlobStorageReadinessBlocker>,
    pub warnings: Vec<BlobStorageBillingWarning>,
}

/// Validate configuration fields that precede boundary-number conversion.
pub fn validate_blob_storage_billing_config_header(
    cashier_canister_id: Principal,
    gateway_principal_limit: u64,
) -> Result<(), BlobStorageBillingPolicyError> {
    if cashier_canister_id == Principal::anonymous()
        || cashier_canister_id == Principal::management_canister()
    {
        return Err(BlobStorageBillingPolicyError::CashierCanisterRequired);
    }
    if gateway_principal_limit == 0 {
        return Err(BlobStorageBillingPolicyError::GatewayPrincipalLimitZero);
    }
    Ok(())
}

/// Validate converted billing balance thresholds.
pub const fn validate_blob_storage_billing_balances(
    project_cycles_reserve: u128,
    min_upload_balance: u128,
    target_upload_balance: u128,
) -> Result<(), BlobStorageBillingPolicyError> {
    if project_cycles_reserve == 0 {
        return Err(BlobStorageBillingPolicyError::ProjectCyclesReserveZero);
    }
    if min_upload_balance == 0 {
        return Err(BlobStorageBillingPolicyError::MinUploadBalanceZero);
    }
    if target_upload_balance == 0 {
        return Err(BlobStorageBillingPolicyError::TargetUploadBalanceZero);
    }
    if min_upload_balance > target_upload_balance {
        return Err(BlobStorageBillingPolicyError::MinUploadBalanceExceedsTarget);
    }
    Ok(())
}

/// Validate an explicit project-cycle funding request.
pub const fn validate_blob_storage_funding_request(
    requested_cycles: u128,
) -> Result<(), BlobStorageBillingPolicyError> {
    if requested_cycles == 0 {
        return Err(BlobStorageBillingPolicyError::RequestedCyclesZero);
    }
    Ok(())
}

/// Decide whether the requested funding can be attached without crossing reserve.
#[must_use]
pub const fn decide_blob_storage_funding_attachment(
    requested_cycles: u128,
    project_cycles_available: u128,
    project_cycles_reserve: u128,
) -> BlobStorageFundingAttachmentDecision {
    let transferable_cycles = project_cycles_available.saturating_sub(project_cycles_reserve);
    let reserve_would_be_violated = requested_cycles > transferable_cycles;
    let attached_cycles = if reserve_would_be_violated {
        0
    } else {
        requested_cycles
    };
    let skipped_reason = if reserve_would_be_violated {
        Some("reserve would be violated")
    } else {
        None
    };

    BlobStorageFundingAttachmentDecision {
        project_cycles_available,
        attached_cycles,
        skipped_reason,
    }
}

/// Decide an unconfigured, read-only billing status.
#[must_use]
pub fn decide_unconfigured_blob_storage_status(
    sync_gateway_principals: bool,
) -> BlobStorageStatusDecision {
    BlobStorageStatusDecision {
        gateway_principal_sync_action: if sync_gateway_principals {
            BlobStorageGatewayPrincipalSyncAction::SkippedConfigMissing
        } else {
            BlobStorageGatewayPrincipalSyncAction::NotRequested
        },
        funding_status: BlobStorageFundingStatusDecision::NotConfigured,
        ready: false,
        blockers: vec![BlobStorageReadinessBlocker::NotConfigured],
        warnings: Vec::new(),
    }
}

/// Decide configured billing readiness from observed local and Cashier state.
#[must_use]
pub fn decide_configured_blob_storage_status(
    sync_gateway_principals: bool,
    gateway_principal_count: u64,
    balance: BlobStorageCashierBalanceObservation,
    min_upload_balance: u128,
    target_upload_balance: u128,
    project_cycles_reserve: u128,
    project_cycles_available: u128,
) -> BlobStorageStatusDecision {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if sync_gateway_principals {
        warnings.push(BlobStorageBillingWarning::SyncRequestedButStatusIsReadOnly);
    }
    if gateway_principal_count == 0 {
        blockers.push(BlobStorageReadinessBlocker::GatewayPrincipalsMissing);
        warnings.push(BlobStorageBillingWarning::GatewayPrincipalSetEmpty);
    }

    let funding_status = match balance {
        BlobStorageCashierBalanceObservation::Malformed => {
            blockers.push(BlobStorageReadinessBlocker::CashierBalanceMalformed);
            warnings.push(BlobStorageBillingWarning::CashierBalanceMalformed);
            BlobStorageFundingStatusDecision::BalanceMalformed
        }
        BlobStorageCashierBalanceObservation::Unavailable => {
            blockers.push(BlobStorageReadinessBlocker::CashierBalanceUnavailable);
            warnings.push(BlobStorageBillingWarning::CashierBalanceUnavailable);
            BlobStorageFundingStatusDecision::BalanceUnavailable
        }
        BlobStorageCashierBalanceObservation::Available(balance)
            if balance >= min_upload_balance =>
        {
            BlobStorageFundingStatusDecision::NotNeeded
        }
        BlobStorageCashierBalanceObservation::Available(balance) => {
            blockers.push(BlobStorageReadinessBlocker::InsufficientCashierBalance);
            let requested_cycles = target_upload_balance.saturating_sub(balance);
            let transferable_cycles =
                project_cycles_available.saturating_sub(project_cycles_reserve);
            if requested_cycles > transferable_cycles {
                blockers.push(BlobStorageReadinessBlocker::ReserveWouldBeViolated);
                BlobStorageFundingStatusDecision::ReserveWouldBeViolated {
                    requested_cycles,
                    transferable_cycles,
                }
            } else {
                BlobStorageFundingStatusDecision::FundingRequired { requested_cycles }
            }
        }
    };

    BlobStorageStatusDecision {
        gateway_principal_sync_action: if sync_gateway_principals {
            BlobStorageGatewayPrincipalSyncAction::SkippedReadOnlyStatus
        } else {
            BlobStorageGatewayPrincipalSyncAction::NotRequested
        },
        funding_status,
        ready: blockers.is_empty(),
        blockers,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn billing_config_policy_accepts_valid_values_and_rejects_each_boundary() {
        assert_eq!(validate_blob_storage_billing_config_header(p(1), 8), Ok(()));
        assert_eq!(validate_blob_storage_billing_balances(1, 10, 100), Ok(()));

        assert_eq!(
            validate_blob_storage_billing_config_header(Principal::anonymous(), 8),
            Err(BlobStorageBillingPolicyError::CashierCanisterRequired)
        );
        assert_eq!(
            validate_blob_storage_billing_config_header(Principal::management_canister(), 8),
            Err(BlobStorageBillingPolicyError::CashierCanisterRequired)
        );
        assert_eq!(
            validate_blob_storage_billing_config_header(p(1), 0),
            Err(BlobStorageBillingPolicyError::GatewayPrincipalLimitZero)
        );
        assert_eq!(
            validate_blob_storage_billing_balances(0, 10, 100),
            Err(BlobStorageBillingPolicyError::ProjectCyclesReserveZero)
        );
        assert_eq!(
            validate_blob_storage_billing_balances(1, 0, 100),
            Err(BlobStorageBillingPolicyError::MinUploadBalanceZero)
        );
        assert_eq!(
            validate_blob_storage_billing_balances(1, 10, 0),
            Err(BlobStorageBillingPolicyError::TargetUploadBalanceZero)
        );
        assert_eq!(
            validate_blob_storage_billing_balances(1, 100, 10),
            Err(BlobStorageBillingPolicyError::MinUploadBalanceExceedsTarget)
        );
    }

    #[test]
    fn funding_policy_rejects_zero_and_never_partially_attaches() {
        assert_eq!(
            validate_blob_storage_funding_request(0),
            Err(BlobStorageBillingPolicyError::RequestedCyclesZero)
        );
        assert_eq!(
            decide_blob_storage_funding_attachment(500, 1_000, 500),
            BlobStorageFundingAttachmentDecision {
                project_cycles_available: 1_000,
                attached_cycles: 500,
                skipped_reason: None,
            }
        );
        assert_eq!(
            decide_blob_storage_funding_attachment(500, 1_000, 700),
            BlobStorageFundingAttachmentDecision {
                project_cycles_available: 1_000,
                attached_cycles: 0,
                skipped_reason: Some("reserve would be violated"),
            }
        );
    }

    #[test]
    fn status_policy_covers_unconfigured_ready_required_and_reserve_states() {
        let unconfigured = decide_unconfigured_blob_storage_status(true);
        assert_eq!(
            unconfigured.gateway_principal_sync_action,
            BlobStorageGatewayPrincipalSyncAction::SkippedConfigMissing
        );
        assert_eq!(
            unconfigured.blockers,
            vec![BlobStorageReadinessBlocker::NotConfigured]
        );

        let ready = decide_configured_blob_storage_status(
            false,
            1,
            BlobStorageCashierBalanceObservation::Available(10),
            10,
            100,
            1,
            1_000,
        );
        assert!(ready.ready);
        assert_eq!(
            ready.funding_status,
            BlobStorageFundingStatusDecision::NotNeeded
        );

        let required = decide_configured_blob_storage_status(
            false,
            1,
            BlobStorageCashierBalanceObservation::Available(9),
            10,
            100,
            1,
            1_000,
        );
        assert_eq!(
            required.funding_status,
            BlobStorageFundingStatusDecision::FundingRequired {
                requested_cycles: 91,
            }
        );

        let reserve = decide_configured_blob_storage_status(
            true,
            0,
            BlobStorageCashierBalanceObservation::Available(9),
            10,
            100,
            950,
            1_000,
        );
        assert_eq!(
            reserve.funding_status,
            BlobStorageFundingStatusDecision::ReserveWouldBeViolated {
                requested_cycles: 91,
                transferable_cycles: 50,
            }
        );
        assert_eq!(
            reserve.blockers,
            vec![
                BlobStorageReadinessBlocker::GatewayPrincipalsMissing,
                BlobStorageReadinessBlocker::InsufficientCashierBalance,
                BlobStorageReadinessBlocker::ReserveWouldBeViolated,
            ]
        );
    }

    #[test]
    fn status_policy_distinguishes_malformed_and_unavailable_cashier_balances() {
        let malformed = decide_configured_blob_storage_status(
            false,
            1,
            BlobStorageCashierBalanceObservation::Malformed,
            10,
            100,
            1,
            1_000,
        );
        assert_eq!(
            malformed.funding_status,
            BlobStorageFundingStatusDecision::BalanceMalformed
        );

        let unavailable = decide_configured_blob_storage_status(
            false,
            1,
            BlobStorageCashierBalanceObservation::Unavailable,
            10,
            100,
            1,
            1_000,
        );
        assert_eq!(
            unavailable.funding_status,
            BlobStorageFundingStatusDecision::BalanceUnavailable
        );
    }
}
