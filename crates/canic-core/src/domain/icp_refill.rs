//! Module: domain::icp_refill
//!
//! Responsibility: define pure ICP refill operation value enums shared across
//! storage projections, workflow decisions, and endpoint DTOs.
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
