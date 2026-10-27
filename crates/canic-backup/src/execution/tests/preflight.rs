//! Module: execution::tests::preflight
//!
//! Responsibility: initial execution journal preflight state tests.
//! Does not own: operation receipt validation.
//! Boundary: preflight identity and mutating-operation gates.

use super::*;

// Ensure new journals keep mutating work blocked until preflight evidence is accepted.
#[test]
fn journal_blocks_mutation_until_preflight_is_accepted() {
    let journal = journal();

    assert!(!journal.preflight_accepted);
    assert_eq!(
        journal.next_ready_operation().expect("next op").kind,
        BackupOperationKind::ValidateTopology
    );
    assert!(
        journal
            .operations
            .iter()
            .filter(|operation| operation.kind == BackupOperationKind::Stop)
            .all(|operation| operation.state == BackupExecutionOperationState::Blocked)
    );
}
