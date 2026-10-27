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
