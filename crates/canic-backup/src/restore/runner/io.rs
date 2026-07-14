//! Module: restore::runner::io
//!
//! Responsibility: read, write, and receipt-check restore apply journals.
//! Does not own: command execution, response rendering, or restore plan construction.
//! Boundary: filesystem adapter for native restore runner journal state.

use super::{
    RestoreApplyJournal, RestoreApplyOperationReceiptOutcome, RestoreApplyOperationState,
    types::RestoreRunnerError,
};
use crate::restore::write_restore_apply_journal;
use std::{fs, path::Path};

pub(super) fn read_apply_journal_file(
    path: &Path,
) -> Result<RestoreApplyJournal, RestoreRunnerError> {
    let data = fs::read_to_string(path)?;
    let journal: RestoreApplyJournal = serde_json::from_str(&data)?;
    journal.validate()?;
    validate_terminal_operation_receipts(&journal)?;
    Ok(journal)
}

pub(super) fn write_apply_journal_file(
    path: &Path,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreRunnerError> {
    write_restore_apply_journal(path, journal)?;
    Ok(())
}

fn validate_terminal_operation_receipts(
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreRunnerError> {
    for operation in journal.operations.iter().filter(|operation| {
        matches!(
            operation.state,
            RestoreApplyOperationState::Completed | RestoreApplyOperationState::Failed
        )
    }) {
        let expected_outcome = receipt_outcome_for_state(&operation.state);
        let latest_receipt = journal
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == operation.sequence)
            .max_by_key(|receipt| receipt.attempt);
        let Some(receipt) = latest_receipt else {
            return Err(RestoreRunnerError::TerminalOperationMissingReceipt {
                backup_id: journal.backup_id.clone(),
                sequence: operation.sequence,
                state: operation_state_label(&operation.state),
            });
        };

        let receipt_matches = receipt.operation == operation.operation
            && receipt.source_canister == operation.source_canister
            && receipt.target_canister == operation.target_canister
            && receipt.outcome == expected_outcome
            && receipt.updated_at.as_deref() == operation.state_updated_at.as_deref();
        if !receipt_matches {
            return Err(RestoreRunnerError::TerminalOperationReceiptMismatch {
                backup_id: journal.backup_id.clone(),
                sequence: operation.sequence,
                state: operation_state_label(&operation.state),
            });
        }
    }

    Ok(())
}

fn receipt_outcome_for_state(
    state: &RestoreApplyOperationState,
) -> RestoreApplyOperationReceiptOutcome {
    match state {
        RestoreApplyOperationState::Completed => {
            RestoreApplyOperationReceiptOutcome::CommandCompleted
        }
        RestoreApplyOperationState::Failed => RestoreApplyOperationReceiptOutcome::CommandFailed,
        RestoreApplyOperationState::Blocked
        | RestoreApplyOperationState::Pending
        | RestoreApplyOperationState::Ready => {
            unreachable!("non-terminal restore operation state has no receipt outcome")
        }
    }
}

const fn operation_state_label(state: &RestoreApplyOperationState) -> &'static str {
    match state {
        RestoreApplyOperationState::Completed => "completed",
        RestoreApplyOperationState::Failed => "failed",
        RestoreApplyOperationState::Blocked
        | RestoreApplyOperationState::Pending
        | RestoreApplyOperationState::Ready => "non-terminal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restore::{
        RestoreApplyCommandOutputPair, RestoreApplyJournalOperation, RestoreApplyOperationKind,
        RestoreApplyOperationKindCounts, RestoreApplyOperationReceipt, RestoreApplyRunnerCommand,
    };

    const BACKUP_ID: &str = "backup-test";
    const SOURCE: &str = "aaaaa-aa";
    const TARGET: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";

    #[test]
    fn terminal_receipts_must_match_latest_attempt() {
        let mut journal = terminal_journal(RestoreApplyOperationState::Completed, "unix:2");
        let operation = journal.operations[0].clone();
        journal
            .operation_receipts
            .push(RestoreApplyOperationReceipt::command_completed(
                &operation,
                runner_command(),
                "0".to_string(),
                Some("unix:1".to_string()),
                command_output_pair(),
                1,
                None,
            ));
        journal
            .operation_receipts
            .push(RestoreApplyOperationReceipt::command_failed(
                &operation,
                runner_command(),
                "1".to_string(),
                Some("unix:2".to_string()),
                command_output_pair(),
                2,
                "icp-failed".to_string(),
            ));

        let err = validate_terminal_operation_receipts(&journal)
            .expect_err("latest mismatched receipt should reject");

        std::assert_matches!(
            err,
            RestoreRunnerError::TerminalOperationReceiptMismatch {
                sequence: 0,
                state: "completed",
                ..
            }
        );
    }

    #[test]
    fn terminal_receipts_must_match_state_timestamp() {
        let mut journal = terminal_journal(RestoreApplyOperationState::Completed, "unix:2");
        let operation = journal.operations[0].clone();
        journal
            .operation_receipts
            .push(RestoreApplyOperationReceipt::command_completed(
                &operation,
                runner_command(),
                "0".to_string(),
                Some("unix:1".to_string()),
                command_output_pair(),
                1,
                None,
            ));

        let err = validate_terminal_operation_receipts(&journal)
            .expect_err("stale receipt timestamp should reject");

        std::assert_matches!(
            err,
            RestoreRunnerError::TerminalOperationReceiptMismatch {
                sequence: 0,
                state: "completed",
                ..
            }
        );
    }

    #[test]
    fn terminal_receipts_accept_latest_matching_attempt() {
        let mut journal = terminal_journal(RestoreApplyOperationState::Completed, "unix:2");
        let operation = journal.operations[0].clone();
        journal
            .operation_receipts
            .push(RestoreApplyOperationReceipt::command_failed(
                &operation,
                runner_command(),
                "1".to_string(),
                Some("unix:1".to_string()),
                command_output_pair(),
                1,
                "icp-failed".to_string(),
            ));
        journal
            .operation_receipts
            .push(RestoreApplyOperationReceipt::command_completed(
                &operation,
                runner_command(),
                "0".to_string(),
                Some("unix:2".to_string()),
                command_output_pair(),
                2,
                None,
            ));

        validate_terminal_operation_receipts(&journal)
            .expect("latest matching receipt should validate");
    }

    fn terminal_journal(
        state: RestoreApplyOperationState,
        updated_at: &str,
    ) -> RestoreApplyJournal {
        RestoreApplyJournal {
            journal_version: 1,
            backup_id: BACKUP_ID.to_string(),
            ready: true,
            blocked_reasons: Vec::new(),
            backup_root: None,
            operation_count: 1,
            operation_counts: RestoreApplyOperationKindCounts::default(),
            pending_operations: 0,
            ready_operations: 0,
            blocked_operations: 0,
            completed_operations: usize::from(state == RestoreApplyOperationState::Completed),
            failed_operations: usize::from(state == RestoreApplyOperationState::Failed),
            operations: vec![RestoreApplyJournalOperation {
                sequence: 0,
                operation: RestoreApplyOperationKind::StartCanister,
                state,
                state_updated_at: Some(updated_at.to_string()),
                blocking_reasons: Vec::new(),
                member_order: 0,
                source_canister: SOURCE.to_string(),
                target_canister: TARGET.to_string(),
                role: "root".to_string(),
                snapshot_id: None,
                artifact_path: None,
                verification_kind: None,
            }],
            operation_receipts: Vec::new(),
        }
    }

    fn runner_command() -> RestoreApplyRunnerCommand {
        RestoreApplyRunnerCommand {
            program: "icp".to_string(),
            args: vec![
                "canister".to_string(),
                "start".to_string(),
                TARGET.to_string(),
            ],
            mutates: true,
            requires_stopped_canister: false,
            note: "starts target canister".to_string(),
        }
    }

    fn command_output_pair() -> RestoreApplyCommandOutputPair {
        RestoreApplyCommandOutputPair::from_bytes(b"ok\n", b"", 1024)
    }
}
