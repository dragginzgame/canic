use super::{
    RestoreApplyJournal, RestoreApplyOperationReceiptOutcome, RestoreApplyOperationState,
    types::RestoreRunnerError,
};
use crate::timestamp::current_timestamp_marker;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

// Read and validate a restore apply journal from disk.
pub(super) fn read_apply_journal_file(
    path: &Path,
) -> Result<RestoreApplyJournal, RestoreRunnerError> {
    let data = fs::read_to_string(path)?;
    let journal: RestoreApplyJournal = serde_json::from_str(&data)?;
    journal.validate()?;
    validate_terminal_operation_receipts(&journal)?;
    Ok(journal)
}

// Return the caller-supplied journal update marker or the current timestamp.
pub(super) fn state_updated_at(updated_at: Option<&String>) -> String {
    updated_at.cloned().unwrap_or_else(current_timestamp_marker)
}

// Persist the restore apply journal to its canonical runner path.
pub(super) fn write_apply_journal_file(
    path: &Path,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreRunnerError> {
    let data = serde_json::to_vec_pretty(journal)?;
    fs::write(path, data)?;
    Ok(())
}

///
/// RestoreJournalLock
///

pub(super) struct RestoreJournalLock {
    path: PathBuf,
}

impl RestoreJournalLock {
    // Acquire an atomic sidecar lock for mutating restore runner operations.
    pub(super) fn acquire(journal_path: &Path) -> Result<Self, RestoreRunnerError> {
        let path = journal_lock_path(journal_path);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                writeln!(file, "pid={}", std::process::id())?;
                Ok(Self { path })
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(RestoreRunnerError::JournalLocked {
                    lock_path: path.to_string_lossy().to_string(),
                })
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for RestoreJournalLock {
    // Release the sidecar lock when the mutating command completes or fails.
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

// Derive the sidecar lock path for one apply journal.
fn journal_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}

// Ensure terminal restore-runner state is backed by a durable command receipt.
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
        let has_receipt = journal.operation_receipts.iter().any(|receipt| {
            receipt.sequence == operation.sequence
                && receipt.operation == operation.operation
                && receipt.source_canister == operation.source_canister
                && receipt.target_canister == operation.target_canister
                && receipt.outcome == expected_outcome
        });
        if !has_receipt {
            return Err(RestoreRunnerError::TerminalOperationMissingReceipt {
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
