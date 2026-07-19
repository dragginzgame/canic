//! Module: execution::tests::receipts
//!
//! Responsibility: execution operation receipt validation tests.
//! Does not own: generic transition ordering.
//! Boundary: receipt-required fields and failed mutation rollback.

use super::*;

#[test]
fn operation_receipt_requires_current_snapshot_metadata_fields() {
    let journal = accepted_journal();
    let operation = journal.operations[4].clone();
    let receipt = BackupExecutionOperationReceipt::completed(
        &journal,
        &operation,
        Some("unix:31".to_string()),
    );

    for field in ["snapshot_taken_at_timestamp", "snapshot_total_size_bytes"] {
        let mut value = serde_json::to_value(&receipt).expect("serialize receipt");
        value.as_object_mut().expect("receipt object").remove(field);

        let err = serde_json::from_value::<BackupExecutionOperationReceipt>(value)
            .expect_err("current snapshot metadata field must be present");

        assert!(err.is_data());
    }
}

// Ensure snapshot creation receipts must carry the created snapshot id.
#[test]
fn snapshot_completion_requires_snapshot_id() {
    let mut journal = accepted_journal();
    complete_operation(&mut journal, 4);
    journal
        .mark_operation_pending_at(5, Some("unix:30".to_string()))
        .expect("mark snapshot pending");
    let operation = journal.operations[5].clone();
    let receipt = BackupExecutionOperationReceipt::completed(
        &journal,
        &operation,
        Some("unix:31".to_string()),
    );
    let receipt_count = journal.operation_receipts.len();

    let err = journal
        .record_operation_receipt(receipt)
        .expect_err("missing snapshot id rejects");

    assert!(
        matches!(err, BackupExecutionJournalError::MissingField(field) if field == "operation_receipts[].snapshot_id")
    );
    assert_eq!(journal.operation_receipts.len(), receipt_count);
    assert_eq!(
        journal.operations[5].state,
        BackupExecutionOperationState::Pending
    );
    assert_eq!(
        journal.operations[5].state_updated_at.as_deref(),
        Some("unix:30")
    );
    assert!(journal.operations[5].blocking_reasons.is_empty());
}

// Ensure operation receipts always carry an audit timestamp.
#[test]
fn operation_receipts_require_updated_at() {
    let mut journal = accepted_journal();
    journal
        .mark_operation_pending_at(4, Some("unix:40".to_string()))
        .expect("mark stop pending");
    let operation = journal.operations[4].clone();
    let receipt = BackupExecutionOperationReceipt::completed(&journal, &operation, None);

    let err = journal
        .record_operation_receipt(receipt)
        .expect_err("missing receipt timestamp rejects");

    assert!(
        matches!(err, BackupExecutionJournalError::MissingField(field) if field == "operation_receipts[].updated_at")
    );
    assert!(journal.operation_receipts.is_empty());
    assert_eq!(
        journal.operations[4].state,
        BackupExecutionOperationState::Pending
    );
    assert_eq!(
        journal.operations[4].state_updated_at.as_deref(),
        Some("unix:40")
    );
    assert!(journal.operations[4].blocking_reasons.is_empty());
}
