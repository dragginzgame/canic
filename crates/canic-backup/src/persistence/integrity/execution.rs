//! Module: persistence::integrity::execution
//!
//! Responsibility: verify execution journals are bound to backup plans.
//! Does not own: plan construction, journal mutation, or restore execution.
//! Boundary: returns typed execution integrity reports or persistence errors.

use crate::{
    execution::{
        BackupExecutionJournal, BackupExecutionOperationReceiptOutcome,
        BackupExecutionOperationState,
    },
    persistence::{BackupExecutionIntegrityReport, PersistenceError},
    plan::{BackupOperationKind, BackupPlan},
};

pub(in crate::persistence) fn verify_execution_integrity(
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
) -> Result<BackupExecutionIntegrityReport, PersistenceError> {
    if plan.plan_id != journal.plan_id {
        return Err(PersistenceError::PlanJournalMismatch {
            field: "plan_id",
            plan: plan.plan_id.clone(),
            journal: journal.plan_id.clone(),
        });
    }
    if plan.run_id != journal.run_id {
        return Err(PersistenceError::PlanJournalMismatch {
            field: "run_id",
            plan: plan.run_id.clone(),
            journal: journal.run_id.clone(),
        });
    }
    if plan.phases.len() != journal.operations.len() {
        return Err(PersistenceError::PlanJournalMismatch {
            field: "operation_count",
            plan: plan.phases.len().to_string(),
            journal: journal.operations.len().to_string(),
        });
    }

    for (phase, operation) in plan.phases.iter().zip(&journal.operations) {
        let expected_sequence = usize::try_from(phase.order).unwrap_or(usize::MAX);
        if expected_sequence != operation.sequence {
            return Err(PersistenceError::PlanJournalOperationMismatch {
                sequence: operation.sequence,
                field: "sequence",
                plan: expected_sequence.to_string(),
                journal: operation.sequence.to_string(),
            });
        }
        if phase.operation_id != operation.operation_id {
            return Err(PersistenceError::PlanJournalOperationMismatch {
                sequence: operation.sequence,
                field: "operation_id",
                plan: phase.operation_id.clone(),
                journal: operation.operation_id.clone(),
            });
        }
        if phase.kind != operation.kind {
            return Err(PersistenceError::PlanJournalOperationMismatch {
                sequence: operation.sequence,
                field: "kind",
                plan: format!("{:?}", phase.kind),
                journal: format!("{:?}", operation.kind),
            });
        }
        if phase.target_canister_id != operation.target_canister_id {
            return Err(PersistenceError::PlanJournalOperationMismatch {
                sequence: operation.sequence,
                field: "target_canister_id",
                plan: phase.target_canister_id.clone().unwrap_or_default(),
                journal: operation.target_canister_id.clone().unwrap_or_default(),
            });
        }
    }
    verify_terminal_mutation_receipts(journal)?;

    Ok(BackupExecutionIntegrityReport {
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        verified: true,
        plan_operations: plan.phases.len(),
        journal_operations: journal.operations.len(),
    })
}

fn verify_terminal_mutation_receipts(
    journal: &BackupExecutionJournal,
) -> Result<(), PersistenceError> {
    for operation in journal.operations.iter().filter(|operation| {
        operation_kind_requires_receipt(&operation.kind)
            && matches!(
                operation.state,
                BackupExecutionOperationState::Completed
                    | BackupExecutionOperationState::Failed
                    | BackupExecutionOperationState::Skipped
            )
    }) {
        let expected_outcome = receipt_outcome_for_state(&operation.state);
        let latest_receipt = journal
            .operation_receipts
            .iter()
            .rev()
            .find(|receipt| receipt.sequence == operation.sequence);
        let Some(latest_receipt) = latest_receipt else {
            return Err(PersistenceError::ExecutionOperationMissingReceipt {
                sequence: operation.sequence,
                state: format!("{:?}", operation.state),
            });
        };
        let latest_matches = latest_receipt.operation_id == operation.operation_id
            && latest_receipt.kind == operation.kind
            && latest_receipt.target_canister_id == operation.target_canister_id
            && latest_receipt.outcome == expected_outcome;
        if !latest_matches {
            return Err(PersistenceError::ExecutionOperationMissingReceipt {
                sequence: operation.sequence,
                state: format!("{:?}", operation.state),
            });
        }
        if latest_receipt.updated_at.as_deref() != operation.state_updated_at.as_deref() {
            return Err(
                PersistenceError::ExecutionOperationReceiptTimestampMismatch {
                    sequence: operation.sequence,
                },
            );
        }
    }

    Ok(())
}

const fn operation_kind_requires_receipt(kind: &BackupOperationKind) -> bool {
    matches!(
        kind,
        BackupOperationKind::Stop
            | BackupOperationKind::CreateSnapshot
            | BackupOperationKind::Start
            | BackupOperationKind::DownloadSnapshot
            | BackupOperationKind::VerifyArtifact
            | BackupOperationKind::FinalizeManifest
    )
}

fn receipt_outcome_for_state(
    state: &BackupExecutionOperationState,
) -> BackupExecutionOperationReceiptOutcome {
    match state {
        BackupExecutionOperationState::Completed => {
            BackupExecutionOperationReceiptOutcome::Completed
        }
        BackupExecutionOperationState::Failed => BackupExecutionOperationReceiptOutcome::Failed,
        BackupExecutionOperationState::Skipped => BackupExecutionOperationReceiptOutcome::Skipped,
        BackupExecutionOperationState::Ready
        | BackupExecutionOperationState::Pending
        | BackupExecutionOperationState::Blocked => {
            unreachable!("non-terminal operation state does not have a receipt outcome")
        }
    }
}
