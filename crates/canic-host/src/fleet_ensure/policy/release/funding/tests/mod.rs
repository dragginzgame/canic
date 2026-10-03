//! Receipt facts distinguish completed history from unresolved paid effects and residual review.

use super::*;

fn facts(status: IcpRefillStatus) -> ReleaseRefillFacts {
    ReleaseRefillFacts {
        transfer_uncertain: true,
        record_id: 1,
        operation_id: [2; 32],
        status,
        error_code: None,
        ledger_block_index: None,
        cycles: RecordedRefillCycles::Missing,
        refund_block_index: None,
        expired_before_block: None,
    }
}

#[test]
fn completed_history_preserves_the_exact_conversion_without_reconciliation() {
    let mut history = facts(IcpRefillStatus::Completed);
    history.transfer_uncertain = false;
    history.ledger_block_index = Some(7);
    for cycles in [0, 1, u128::MAX] {
        history.cycles = RecordedRefillCycles::Recorded(cycles);
        assert_eq!(
            assess_refill(&history),
            RecordedConversion {
                ledger_block: 7,
                cycles
            }
        );
    }
}

#[test]
fn recorded_refund_and_missing_refund_block_have_distinct_dispositions() {
    let mut history = facts(IcpRefillStatus::Refunded);
    history.transfer_uncertain = false;
    history.error_code = Some(IcpRefillErrorCode::Refunded);
    history.ledger_block_index = Some(7);
    assert_eq!(
        assess_refill(&history),
        RefundResidualReview { ledger_block: 7 }
    );
    history.refund_block_index = Some(8);
    assert_eq!(
        assess_refill(&history),
        RecordedRefund {
            ledger_block: 7,
            refund_block: 8
        }
    );
}

#[test]
fn malformed_terminal_receipts_never_become_zero_or_settled() {
    let mut history = facts(IcpRefillStatus::Completed);
    history.transfer_uncertain = false;
    assert_eq!(assess_refill(&history), InvalidReceipt);
    history.ledger_block_index = Some(7);
    assert_eq!(assess_refill(&history), InvalidReceipt);
    history.cycles = RecordedRefillCycles::Overflow;
    assert_eq!(assess_refill(&history), InvalidReceipt);
    history.cycles = RecordedRefillCycles::Recorded(1);
    for change in [
        |h: &mut ReleaseRefillFacts| h.transfer_uncertain = true,
        |h: &mut ReleaseRefillFacts| h.refund_block_index = Some(8),
        |h: &mut ReleaseRefillFacts| h.expired_before_block = Some(8),
        |h: &mut ReleaseRefillFacts| h.error_code = Some(IcpRefillErrorCode::NotifyFailed),
    ] {
        let mut wrong = history.clone();
        change(&mut wrong);
        assert_eq!(assess_refill(&wrong), InvalidReceipt);
    }
    history.status = IcpRefillStatus::Refunded;
    history.error_code = Some(IcpRefillErrorCode::Refunded);
    history.refund_block_index = Some(8);
    assert_eq!(assess_refill(&history), InvalidReceipt);
    history.cycles = RecordedRefillCycles::Missing;
    history.ledger_block_index = None;
    assert_eq!(assess_refill(&history), InvalidReceipt);
}

#[test]
fn exhaustion_expiry_and_rejections_do_not_erase_a_potential_earlier_debit() {
    for error in [
        IcpRefillErrorCode::NotifyMaxAttempts,
        IcpRefillErrorCode::TransferWindowStale,
        IcpRefillErrorCode::LedgerTransferFailed,
        IcpRefillErrorCode::BadFee,
        IcpRefillErrorCode::InvalidLedgerBlockIndex,
        IcpRefillErrorCode::CyclesSentOverflow,
    ] {
        let mut unresolved = facts(IcpRefillStatus::Failed);
        unresolved.error_code = Some(error);
        assert_eq!(assess_refill(&unresolved), LedgerReconciliation);
        unresolved.ledger_block_index = Some(7);
        assert_eq!(
            assess_refill(&unresolved),
            CmcReconciliation { ledger_block: 7 }
        );
    }
    for status in [
        IcpRefillStatus::Requested,
        IcpRefillStatus::Transferred,
        IcpRefillStatus::NotifyProcessing,
        IcpRefillStatus::InvalidTransaction,
        IcpRefillStatus::TransactionTooOld,
    ] {
        let mut unresolved = facts(status);
        unresolved.ledger_block_index = Some(7);
        assert_eq!(
            assess_refill(&unresolved),
            CmcReconciliation { ledger_block: 7 }
        );
    }
}

#[test]
fn known_unissued_or_refused_transfers_do_not_require_ledger_reconciliation() {
    let mut unissued = facts(IcpRefillStatus::Requested);
    unissued.transfer_uncertain = false;
    assert_eq!(assess_refill(&unissued), NoLedgerTransfer);
    for error in [
        IcpRefillErrorCode::BadFee,
        IcpRefillErrorCode::LedgerTransferFailed,
        IcpRefillErrorCode::TransferWindowStale,
    ] {
        let mut refused = facts(IcpRefillStatus::Failed);
        refused.error_code = Some(error);
        refused.transfer_uncertain = false;
        assert_eq!(assess_refill(&refused), NoLedgerTransfer);
        refused.transfer_uncertain = true;
        assert_eq!(assess_refill(&refused), LedgerReconciliation);
    }
    unissued.status = IcpRefillStatus::Transferred;
    assert_eq!(assess_refill(&unissued), InvalidReceipt);
}
