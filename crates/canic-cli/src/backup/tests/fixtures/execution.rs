//! Module: backup::tests::fixtures::execution
//!
//! Responsibility: construct retained backup execution journal test fixtures.
//! Does not own: backup plan construction or filesystem layout setup.
//! Boundary: explicit records for status and integrity tests.

use super::{HASH, plan::valid_backup_plan};
use canic_backup::{
    execution::{
        BackupExecutionJournal, BackupExecutionOperationReceipt, BackupExecutionOperationState,
    },
    plan::BackupOperationKind,
};

// Build an execution journal after the preflight gate has been accepted.
pub(in crate::backup::tests) fn accepted_execution_journal() -> BackupExecutionJournal {
    let mut journal =
        BackupExecutionJournal::from_plan(&valid_backup_plan()).expect("execution journal");
    journal.preflight_id = Some("preflight-test".to_string());
    journal.preflight_accepted = true;
    for operation in &mut journal.operations {
        operation.blocking_reasons.clear();
        if matches!(
            operation.kind,
            BackupOperationKind::ValidateTopology
                | BackupOperationKind::ValidateControlAuthority
                | BackupOperationKind::ValidateSnapshotReadAuthority
                | BackupOperationKind::ValidateQuiescencePolicy
        ) {
            operation.state = BackupExecutionOperationState::Completed;
            operation.state_updated_at = Some("unix:10".to_string());
        } else {
            operation.state = BackupExecutionOperationState::Ready;
        }
    }
    journal.validate().expect("retained preflight fixture");
    journal
}

// Complete one operation in an execution journal with the fields required by that operation kind.
pub(in crate::backup::tests) fn complete_execution_operation(
    journal: &mut BackupExecutionJournal,
    sequence: usize,
) {
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
    let operation = &mut journal.operations[sequence];
    operation.state = BackupExecutionOperationState::Completed;
    operation.state_updated_at.clone_from(&receipt.updated_at);
    operation.blocking_reasons.clear();
    if operation.kind == BackupOperationKind::CreateSnapshot {
        operation.snapshot_ids_before = Some(Vec::new());
    }
    journal.operation_receipts.push(receipt);
    journal.restart_required = journal.operations.iter().any(|stop| {
        stop.kind == BackupOperationKind::Stop
            && stop.state == BackupExecutionOperationState::Completed
            && journal.operations.iter().any(|start| {
                start.kind == BackupOperationKind::Start
                    && start.target_canister_id == stop.target_canister_id
                    && start.state != BackupExecutionOperationState::Completed
            })
    });
    journal
        .validate()
        .expect("retained completed operation fixture");
}

// Fail one operation in an execution journal.
pub(in crate::backup::tests) fn fail_execution_operation(
    journal: &mut BackupExecutionJournal,
    sequence: usize,
    reason: &str,
) {
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
    let operation = &mut journal.operations[sequence];
    operation.state = BackupExecutionOperationState::Failed;
    operation.state_updated_at.clone_from(&receipt.updated_at);
    operation.blocking_reasons = vec![reason.to_string()];
    if operation.kind == BackupOperationKind::CreateSnapshot {
        operation.snapshot_ids_before = Some(Vec::new());
    }
    journal.operation_receipts.push(receipt);
    journal
        .validate()
        .expect("retained failed operation fixture");
}
