//! Interpret exact funding facts, never retryability or incidental reservation bookkeeping.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::view::release::funding::{
    RecordedRefillCycles, ReleaseRefillDisposition, ReleaseRefillFacts,
};
use ReleaseRefillDisposition::{
    CmcReconciliation, InvalidReceipt, LedgerReconciliation, NoLedgerTransfer, RecordedConversion,
    RecordedRefund, RefundResidualReview,
};
use canic_core::shared_support::icp_refill::{IcpRefillErrorCode, IcpRefillStatus};

/// Classify the retained receipt, not global release readiness or permission to spend again.
pub(in crate::fleet_ensure) fn assess_refill(
    facts: &ReleaseRefillFacts,
) -> ReleaseRefillDisposition {
    match facts.status {
        IcpRefillStatus::Completed => match (facts.ledger_block_index, facts.cycles) {
            (Some(ledger_block), RecordedRefillCycles::Recorded(cycles))
                if !facts.transfer_uncertain
                    && facts.error_code.is_none()
                    && facts.refund_block_index.is_none()
                    && facts.expired_before_block.is_none() =>
            {
                RecordedConversion {
                    ledger_block,
                    cycles,
                }
            }
            _ => InvalidReceipt,
        },
        IcpRefillStatus::Refunded => {
            if facts.transfer_uncertain
                || facts.error_code != Some(IcpRefillErrorCode::Refunded)
                || facts.cycles != RecordedRefillCycles::Missing
                || facts.expired_before_block.is_some()
            {
                return InvalidReceipt;
            }
            match (facts.ledger_block_index, facts.refund_block_index) {
                (Some(ledger_block), Some(refund_block)) => RecordedRefund {
                    ledger_block,
                    refund_block,
                },
                (Some(ledger_block), None) => RefundResidualReview { ledger_block },
                _ => InvalidReceipt,
            }
        }
        _ => {
            if facts.cycles != RecordedRefillCycles::Missing || facts.refund_block_index.is_some() {
                return InvalidReceipt;
            }
            if let Some(ledger_block) = facts.ledger_block_index {
                return CmcReconciliation { ledger_block };
            }
            if facts.transfer_uncertain {
                return LedgerReconciliation;
            }
            if facts.expired_before_block.is_some() {
                return InvalidReceipt;
            }
            match (facts.status, facts.error_code) {
                (IcpRefillStatus::Requested, None)
                | (
                    IcpRefillStatus::Failed,
                    Some(
                        IcpRefillErrorCode::BadFee
                        | IcpRefillErrorCode::LedgerTransferFailed
                        | IcpRefillErrorCode::TransferWindowStale,
                    ),
                ) => NoLedgerTransfer,
                _ => InvalidReceipt,
            }
        }
    }
}
