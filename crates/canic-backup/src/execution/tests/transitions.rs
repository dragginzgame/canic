//! Module: execution::tests::transitions
//!
//! Responsibility: execution operation transition validation tests.
//! Does not own: preflight acceptance or receipt field validation.
//! Boundary: pending/retry transitions and audit timestamps.

use super::*;

// Ensure backup execution transitions always carry audit timestamps.
#[test]
fn operation_transitions_require_updated_at() {
    let mut journal = journal();

    let err = journal
        .accept_preflight_bundle_at(PREFLIGHT_ID.to_string(), None)
        .expect_err("preflight transition timestamp is required");

    assert!(
        matches!(err, BackupExecutionJournalError::MissingField(field) if field == "updated_at")
    );
    assert!(!journal.preflight_accepted);

    let mut journal = accepted_journal();
    let err = journal
        .mark_operation_pending_at(4, None)
        .expect_err("pending transition timestamp is required");

    assert!(
        matches!(err, BackupExecutionJournalError::MissingField(field) if field == "updated_at")
    );
    assert_eq!(
        journal.operations[4].state,
        BackupExecutionOperationState::Ready
    );

    journal
        .mark_operation_pending_at(4, Some("unix:20".to_string()))
        .expect("mark stop pending");
    let operation = journal.operations[4].clone();
    let receipt = BackupExecutionOperationReceipt::failed(
        &journal,
        &operation,
        Some("unix:21".to_string()),
        "stop failed".to_string(),
    );
    journal
        .record_operation_receipt(receipt)
        .expect("record failed stop");
    let err = journal
        .retry_failed_operation_at(4, None)
        .expect_err("retry transition timestamp is required");

    assert!(
        matches!(err, BackupExecutionJournalError::MissingField(field) if field == "updated_at")
    );
    assert_eq!(
        journal.operations[4].state,
        BackupExecutionOperationState::Failed
    );
}

// Ensure persisted pending and terminal operations keep state transition times.
#[test]
fn active_operation_states_require_updated_at() {
    let mut journal = accepted_journal();
    journal.operations[4].state = BackupExecutionOperationState::Pending;

    let err = journal
        .validate()
        .expect_err("pending operation timestamp is required");

    assert!(
        matches!(err, BackupExecutionJournalError::MissingField(field) if field == "operations[].state_updated_at")
    );
}

// Ensure operation execution advances in plan order.
#[test]
fn rejects_out_of_order_mutation() {
    let mut journal = accepted_journal();

    let err = journal
        .mark_operation_pending_at(5, Some("unix:20".to_string()))
        .expect_err("out-of-order operation rejects");

    std::assert_matches!(
        err,
        BackupExecutionJournalError::OutOfOrderOperationTransition {
            requested: 5,
            next: 4
        }
    );
}
