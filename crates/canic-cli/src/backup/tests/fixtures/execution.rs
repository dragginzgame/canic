//! Module: backup::tests::fixtures::execution
//!
//! Responsibility: build and mutate backup execution journal test fixtures.
//! Does not own: backup plan construction or filesystem layout setup.
//! Boundary: deterministic execution-journal state transitions for tests.

use super::{HASH, plan::valid_backup_plan};
use canic_backup::{
    execution::{BackupExecutionJournal, BackupExecutionOperationReceipt},
    plan::BackupOperationKind,
};

// Build an execution journal after the preflight gate has been accepted.
pub(in crate::backup::tests) fn accepted_execution_journal() -> BackupExecutionJournal {
    let mut journal =
        BackupExecutionJournal::from_plan(&valid_backup_plan()).expect("execution journal");
    journal
        .accept_preflight_bundle_at("preflight-test".to_string(), Some("unix:10".to_string()))
        .expect("accept preflight");
    journal
}

// Complete one operation in an execution journal with the fields required by that operation kind.
pub(in crate::backup::tests) fn complete_execution_operation(
    journal: &mut BackupExecutionJournal,
    sequence: usize,
) {
    mark_execution_operation_pending(journal, sequence);
    let operation = journal
        .operations
        .iter()
        .find(|operation| operation.sequence == sequence)
        .expect("operation exists")
        .clone();
    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        &operation,
        Some(format!("unix:{sequence}1")),
    );
    match operation.kind {
        BackupOperationKind::CreateSnapshot => {
            receipt.snapshot_id = Some("snap-app".to_string());
        }
        BackupOperationKind::DownloadSnapshot => {
            receipt.artifact_path = Some("artifacts/app".to_string());
        }
        BackupOperationKind::VerifyArtifact => {
            receipt.checksum = Some(HASH.to_string());
        }
        _ => {}
    }
    journal
        .record_operation_receipt(receipt)
        .expect("record completed operation");
}

// Fail one operation in an execution journal.
pub(in crate::backup::tests) fn fail_execution_operation(
    journal: &mut BackupExecutionJournal,
    sequence: usize,
    reason: &str,
) {
    mark_execution_operation_pending(journal, sequence);
    let operation = journal
        .operations
        .iter()
        .find(|operation| operation.sequence == sequence)
        .expect("operation exists")
        .clone();
    let receipt = BackupExecutionOperationReceipt::failed(
        journal,
        &operation,
        Some(format!("unix:{sequence}1")),
        reason.to_string(),
    );
    journal
        .record_operation_receipt(receipt)
        .expect("record failed operation");
}

fn mark_execution_operation_pending(journal: &mut BackupExecutionJournal, sequence: usize) {
    let operation = journal
        .operations
        .iter()
        .find(|operation| operation.sequence == sequence)
        .expect("operation exists");
    let updated_at = Some(format!("unix:{sequence}0"));
    if operation.kind == BackupOperationKind::CreateSnapshot {
        journal
            .mark_snapshot_create_pending_at(sequence, updated_at, Vec::new())
            .expect("mark snapshot operation pending");
    } else {
        journal
            .mark_operation_pending_at(sequence, updated_at)
            .expect("mark operation pending");
    }
}
