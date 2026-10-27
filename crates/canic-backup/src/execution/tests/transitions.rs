//! Module: execution::tests::transitions
//!
//! Responsibility: retained execution operation validation tests.
//! Does not own: preflight acceptance or receipt field validation.
//! Boundary: snapshot inventory and audit timestamps.

use super::*;

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

#[test]
fn snapshot_pending_claim_requires_exact_inventory_evidence() {
    let mut journal = accepted_journal();
    journal.operations[5].state = BackupExecutionOperationState::Pending;
    journal.operations[5].state_updated_at = Some("unix:20".to_string());

    let error = journal
        .validate()
        .expect_err("snapshot inventory is required");

    std::assert_matches!(
        error,
        BackupExecutionJournalError::MissingField("operations[].snapshot_ids_before")
    );
}

#[test]
fn snapshot_pending_claim_rejects_duplicate_inventory_without_mutation() {
    let mut journal = accepted_journal();
    journal.operations[5].state = BackupExecutionOperationState::Pending;
    journal.operations[5].state_updated_at = Some("unix:20".to_string());
    journal.operations[5].snapshot_ids_before =
        Some(vec!["snapshot-a".to_string(), "snapshot-a".to_string()]);

    let error = journal.validate().expect_err("duplicate inventory rejects");

    std::assert_matches!(error, BackupExecutionJournalError::DuplicateSnapshotIdentity {
        sequence: 5, snapshot_id,
    } if snapshot_id == "snapshot-a");
}
