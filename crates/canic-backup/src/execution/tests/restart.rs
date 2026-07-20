//! Module: execution::tests::restart
//!
//! Responsibility: restart-required and resume summary tests.
//! Does not own: preflight receipt validation.
//! Boundary: restart visibility after stop/start-sensitive operations.

use super::*;

// Ensure completed stop creates an explicit restart-required state.
#[test]
fn completed_stop_sets_restart_required() {
    let mut journal = accepted_journal();

    complete_operation(&mut journal, 4);

    assert!(journal.restart_required);
    let summary = journal.resume_summary();
    assert!(summary.restart_required);
    assert_eq!(
        summary.next_operation.expect("next op").kind,
        BackupOperationKind::CreateSnapshot
    );
}

// Ensure persisted restart-required state cannot drift from completed stop/start state.
#[test]
fn restart_required_must_match_operation_state() {
    let mut journal = accepted_journal();

    complete_operation(&mut journal, 4);
    journal.restart_required = false;

    let err = journal
        .validate()
        .expect_err("restart-required drift rejects");

    std::assert_matches!(err, BackupExecutionJournalError::RestartRequiredMismatch);
}

// Ensure a failed snapshot after stopping leaves restart-required visible and retryable.
#[test]
fn failed_snapshot_after_stop_is_retryable_and_requires_restart() {
    let mut journal = accepted_journal();
    complete_operation(&mut journal, 4);
    journal
        .mark_snapshot_create_pending_at(5, Some("unix:30".to_string()), Vec::new())
        .expect("mark snapshot pending");
    let operation = journal.operations[5].clone();
    let receipt = BackupExecutionOperationReceipt::failed(
        &journal,
        &operation,
        Some("unix:31".to_string()),
        "snapshot create failed".to_string(),
    );

    journal
        .record_operation_receipt(receipt)
        .expect("record snapshot failure");

    assert!(journal.restart_required);
    assert_eq!(
        journal.next_ready_operation().expect("next op").state,
        BackupExecutionOperationState::Failed
    );
    journal
        .retry_failed_operation_at(5, Some("unix:32".to_string()))
        .expect("retry failed snapshot");
    assert_eq!(
        journal.next_ready_operation().expect("next op").state,
        BackupExecutionOperationState::Ready
    );
    assert_eq!(
        journal
            .next_ready_operation()
            .expect("next op")
            .snapshot_ids_before,
        Some(Vec::new())
    );
}
