//! Module: execution::tests::preflight
//!
//! Responsibility: execution journal preflight acceptance tests.
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

// Ensure accepting the preflight bundle completes preflight gates and unblocks mutation.
#[test]
fn accepting_preflight_unblocks_first_mutating_operation() {
    let mut journal = journal();
    let plan = plan();
    let receipts = preflight_receipts(&plan);

    journal
        .accept_preflight_receipts_at(&receipts, Some("unix:10".to_string()))
        .expect("accept preflight");

    assert!(journal.preflight_accepted);
    assert_eq!(journal.preflight_id.as_deref(), Some(PREFLIGHT_ID));
    assert_eq!(
        journal.next_ready_operation().expect("next op").kind,
        BackupOperationKind::Stop
    );
    assert!(
        journal
            .operations
            .iter()
            .filter(|operation| operation.kind == BackupOperationKind::ValidateTopology)
            .all(|operation| operation.state == BackupExecutionOperationState::Completed)
    );
}

// Ensure typed preflight bundles must belong to the same plan as the journal.
#[test]
fn rejects_preflight_receipts_for_different_plan() {
    let mut journal = journal();
    let mut plan = plan();
    plan.plan_id = "plan-other".to_string();
    let receipts = preflight_receipts(&plan);

    let err = journal
        .accept_preflight_receipts_at(&receipts, Some("unix:10".to_string()))
        .expect_err("different plan rejects");

    std::assert_matches!(
        err,
        BackupExecutionJournalError::PreflightPlanMismatch { expected, actual }
            if expected == "plan-001" && actual == "plan-other"
    );
}

// Ensure preflight identity cannot change once accepted.
#[test]
fn rejects_different_preflight_after_acceptance() {
    let mut journal = accepted_journal();

    let err = journal
        .accept_preflight_bundle_at("preflight-other".to_string(), Some("unix:11".to_string()))
        .expect_err("different preflight rejects");

    std::assert_matches!(
        err,
        BackupExecutionJournalError::PreflightAlreadyAccepted { existing, attempted }
            if existing == PREFLIGHT_ID && attempted == "preflight-other"
    );
}
