//! Module: domain::icp_refill
//!
//! Responsibility: define pure ICP refill values and outcome state/budget invariants
//! shared across storage projections, workflow decisions, and endpoint DTOs.
//! Does not own: endpoint request/response structs, stable records, or ledger
//! execution.
//! Boundary: DTOs re-export these values to preserve the public API path while
//! internal code imports them from the domain owner.

pub use canic_contracts::values::icp_refill::{
    IcpRefillErrorCode, IcpRefillStatus, IcpRefillTrigger,
};

/// Return whether an ICP refill outcome must retain its operation identity for retry.
#[must_use]
pub const fn icp_refill_outcome_is_resumable(
    status: IcpRefillStatus,
    error_code: Option<IcpRefillErrorCode>,
    ledger_block_recorded: bool,
) -> bool {
    matches!(
        status,
        IcpRefillStatus::Requested
            | IcpRefillStatus::Transferred
            | IcpRefillStatus::NotifyProcessing
    ) || matches!(
        (status, error_code, ledger_block_recorded),
        (
            IcpRefillStatus::Failed,
            Some(IcpRefillErrorCode::NotifyFailed),
            true
        ) | (
            IcpRefillStatus::Failed,
            Some(IcpRefillErrorCode::BadFee),
            false
        )
    )
}

///
/// IcpRefillOutcome
///
/// Observed outcome whose state and budget consequences have one policy owner.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IcpRefillOutcome {
    Completed,
    Failure(IcpRefillErrorCode),
    InvalidTransaction,
    NotifyProcessing,
    Refunded,
    TransactionTooOld,
    TransferConfirmed,
    TransferDuplicate,
    TransferRejected,
    TransferWindowStale,
}

///
/// IcpRefillOutcomeDecision
///
/// Exact state projection applied by storage ops alongside outcome-specific evidence.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IcpRefillOutcomeDecision {
    pub status: IcpRefillStatus,
    pub error_code: Option<IcpRefillErrorCode>,
    pub transfer_uncertain: bool,
    pub budget_reserved: bool,
}

/// A refusal cannot resolve an earlier uncertain debit. Confirmed transfers clear
/// uncertainty; neither completion nor notification failure restores spending authority.
#[must_use]
pub const fn decide_refill_outcome(
    outcome: IcpRefillOutcome,
    transfer_uncertain: bool,
    budget_reserved: bool,
) -> IcpRefillOutcomeDecision {
    let (status, error_code) = match outcome {
        IcpRefillOutcome::Completed => (IcpRefillStatus::Completed, None),
        IcpRefillOutcome::Failure(code) => (IcpRefillStatus::Failed, Some(code)),
        IcpRefillOutcome::InvalidTransaction => (
            IcpRefillStatus::InvalidTransaction,
            Some(IcpRefillErrorCode::InvalidTransaction),
        ),
        IcpRefillOutcome::NotifyProcessing => (
            IcpRefillStatus::NotifyProcessing,
            Some(IcpRefillErrorCode::Processing),
        ),
        IcpRefillOutcome::Refunded => (
            IcpRefillStatus::Refunded,
            Some(IcpRefillErrorCode::Refunded),
        ),
        IcpRefillOutcome::TransactionTooOld => (
            IcpRefillStatus::TransactionTooOld,
            Some(IcpRefillErrorCode::TransactionTooOld),
        ),
        IcpRefillOutcome::TransferConfirmed => (IcpRefillStatus::Transferred, None),
        IcpRefillOutcome::TransferDuplicate => (
            IcpRefillStatus::Transferred,
            Some(IcpRefillErrorCode::Duplicate),
        ),
        IcpRefillOutcome::TransferRejected => (
            IcpRefillStatus::Failed,
            Some(IcpRefillErrorCode::LedgerTransferFailed),
        ),
        IcpRefillOutcome::TransferWindowStale => (
            IcpRefillStatus::Failed,
            Some(IcpRefillErrorCode::TransferWindowStale),
        ),
    };
    let transfer_uncertain = if matches!(
        outcome,
        IcpRefillOutcome::TransferConfirmed | IcpRefillOutcome::TransferDuplicate
    ) {
        false
    } else {
        transfer_uncertain
    };
    let budget_reserved = if matches!(
        outcome,
        IcpRefillOutcome::TransferRejected | IcpRefillOutcome::TransferWindowStale
    ) {
        transfer_uncertain
    } else {
        budget_reserved
    };
    IcpRefillOutcomeDecision {
        status,
        error_code,
        transfer_uncertain,
        budget_reserved,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_retains_uncertain_spending_and_releases_only_resolved_unspent_budget() {
        for outcome in [
            IcpRefillOutcome::TransferRejected,
            IcpRefillOutcome::TransferWindowStale,
        ] {
            for reserved in [false, true] {
                let uncertain = decide_refill_outcome(outcome, true, reserved);
                assert!(uncertain.transfer_uncertain);
                assert!(uncertain.budget_reserved);
                let resolved = decide_refill_outcome(outcome, false, reserved);
                assert!(!resolved.transfer_uncertain);
                assert!(!resolved.budget_reserved);
                assert_eq!(resolved.status, IcpRefillStatus::Failed);
            }
        }
    }

    #[test]
    fn proven_debits_clear_uncertainty_without_reissuing_budget() {
        for outcome in [
            IcpRefillOutcome::TransferConfirmed,
            IcpRefillOutcome::TransferDuplicate,
        ] {
            let confirmed = decide_refill_outcome(outcome, true, true);
            assert!(!confirmed.transfer_uncertain);
            assert!(confirmed.budget_reserved);
            assert_eq!(confirmed.status, IcpRefillStatus::Transferred);
        }
        for outcome in [
            IcpRefillOutcome::Completed,
            IcpRefillOutcome::Refunded,
            IcpRefillOutcome::NotifyProcessing,
            IcpRefillOutcome::Failure(IcpRefillErrorCode::BadFee),
            IcpRefillOutcome::Failure(IcpRefillErrorCode::NotifyFailed),
        ] {
            for uncertain in [false, true] {
                for reserved in [false, true] {
                    let decision = decide_refill_outcome(outcome, uncertain, reserved);
                    assert_eq!(decision.transfer_uncertain, uncertain);
                    assert_eq!(decision.budget_reserved, reserved);
                }
            }
        }
    }

    #[test]
    fn resumable_outcome_requires_active_or_retryable_state() {
        assert!(icp_refill_outcome_is_resumable(
            IcpRefillStatus::Requested,
            None,
            false
        ));
        assert!(icp_refill_outcome_is_resumable(
            IcpRefillStatus::Transferred,
            None,
            true
        ));
        assert!(icp_refill_outcome_is_resumable(
            IcpRefillStatus::NotifyProcessing,
            Some(IcpRefillErrorCode::Processing),
            true
        ));
        assert!(icp_refill_outcome_is_resumable(
            IcpRefillStatus::Failed,
            Some(IcpRefillErrorCode::BadFee),
            false
        ));
        assert!(icp_refill_outcome_is_resumable(
            IcpRefillStatus::Failed,
            Some(IcpRefillErrorCode::NotifyFailed),
            true
        ));

        assert!(!icp_refill_outcome_is_resumable(
            IcpRefillStatus::Completed,
            None,
            true
        ));
        assert!(!icp_refill_outcome_is_resumable(
            IcpRefillStatus::Failed,
            Some(IcpRefillErrorCode::NotifyMaxAttempts),
            true
        ));
        assert!(!icp_refill_outcome_is_resumable(
            IcpRefillStatus::Failed,
            Some(IcpRefillErrorCode::NotifyFailed),
            false
        ));
        assert!(!icp_refill_outcome_is_resumable(
            IcpRefillStatus::Failed,
            Some(IcpRefillErrorCode::BadFee),
            true
        ));
    }
}
