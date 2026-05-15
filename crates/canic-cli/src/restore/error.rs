use canic_backup::{
    persistence::PersistenceError,
    restore::{
        RestoreApplyDryRunError, RestoreApplyJournalError, RestorePlanError, RestoreRunnerError,
    },
};
use thiserror::Error as ThisError;

///
/// RestoreCommandError
///

#[derive(Debug, ThisError)]
pub enum RestoreCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("--require-verified requires --backup-dir")]
    RequireVerifiedNeedsBackupDir,

    #[error("restore run command failed for operation {sequence}: status={status}")]
    RestoreRunCommandFailed { sequence: usize, status: String },

    #[error("restore apply journal is locked: {lock_path}")]
    RestoreApplyJournalLocked { lock_path: String },

    #[error("restore plan for backup {backup_id} is not restore-ready: reasons={reasons:?}")]
    RestoreNotReady {
        backup_id: String,
        reasons: Vec<String>,
    },

    #[error(
        "restore apply journal for backup {backup_id} has pending operations: pending={pending_operations}, next={next_transition_sequence:?}"
    )]
    RestoreApplyPending {
        backup_id: String,
        pending_operations: usize,
        next_transition_sequence: Option<usize>,
    },

    #[error(
        "restore apply journal for backup {backup_id} is incomplete: completed={completed_operations}, total={operation_count}"
    )]
    RestoreApplyIncomplete {
        backup_id: String,
        completed_operations: usize,
        operation_count: usize,
    },

    #[error(
        "restore apply journal for backup {backup_id} has failed operations: failed={failed_operations}"
    )]
    RestoreApplyFailed {
        backup_id: String,
        failed_operations: usize,
    },

    #[error("restore apply journal for backup {backup_id} is not ready: reasons={reasons:?}")]
    RestoreApplyNotReady {
        backup_id: String,
        reasons: Vec<String>,
    },

    #[error("restore apply report for backup {backup_id} requires attention: outcome={outcome:?}")]
    RestoreApplyReportNeedsAttention {
        backup_id: String,
        outcome: canic_backup::restore::RestoreApplyReportOutcome,
    },

    #[error(
        "restore apply journal for backup {backup_id} has no executable command: operation_available={operation_available}, complete={complete}, blocked_reasons={blocked_reasons:?}"
    )]
    RestoreApplyCommandUnavailable {
        backup_id: String,
        operation_available: bool,
        complete: bool,
        blocked_reasons: Vec<String>,
    },

    #[error(
        "restore apply journal next operation changed before claim: expected={expected}, actual={actual:?}"
    )]
    RestoreRunClaimSequenceMismatch {
        expected: usize,
        actual: Option<usize>,
    },

    #[error(
        "restore apply journal for backup {backup_id} operation {sequence} is {state} but has no matching receipt"
    )]
    RestoreRunTerminalOperationMissingReceipt {
        backup_id: String,
        sequence: usize,
        state: String,
    },

    #[error(
        "restore apply journal for backup {backup_id} operation {sequence} is {state} but latest receipt is stale or mismatched"
    )]
    RestoreRunTerminalOperationReceiptMismatch {
        backup_id: String,
        sequence: usize,
        state: String,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    RestorePlan(#[from] RestorePlanError),

    #[error(transparent)]
    RestoreApplyDryRun(#[from] RestoreApplyDryRunError),

    #[error(transparent)]
    RestoreApplyJournal(#[from] RestoreApplyJournalError),
}

impl From<RestoreRunnerError> for RestoreCommandError {
    fn from(error: RestoreRunnerError) -> Self {
        match error {
            RestoreRunnerError::CommandFailed { sequence, status } => {
                Self::RestoreRunCommandFailed { sequence, status }
            }
            RestoreRunnerError::JournalLocked { lock_path } => {
                Self::RestoreApplyJournalLocked { lock_path }
            }
            RestoreRunnerError::Pending {
                backup_id,
                pending_operations,
                next_transition_sequence,
            } => Self::RestoreApplyPending {
                backup_id,
                pending_operations,
                next_transition_sequence,
            },
            RestoreRunnerError::Failed {
                backup_id,
                failed_operations,
            } => Self::RestoreApplyFailed {
                backup_id,
                failed_operations,
            },
            RestoreRunnerError::NotReady { backup_id, reasons } => {
                Self::RestoreApplyNotReady { backup_id, reasons }
            }
            RestoreRunnerError::CommandUnavailable {
                backup_id,
                operation_available,
                complete,
                blocked_reasons,
            } => Self::RestoreApplyCommandUnavailable {
                backup_id,
                operation_available,
                complete,
                blocked_reasons,
            },
            RestoreRunnerError::ClaimSequenceMismatch { expected, actual } => {
                Self::RestoreRunClaimSequenceMismatch { expected, actual }
            }
            RestoreRunnerError::TerminalOperationMissingReceipt {
                backup_id,
                sequence,
                state,
            } => Self::RestoreRunTerminalOperationMissingReceipt {
                backup_id,
                sequence,
                state: state.to_string(),
            },
            RestoreRunnerError::TerminalOperationReceiptMismatch {
                backup_id,
                sequence,
                state,
            } => Self::RestoreRunTerminalOperationReceiptMismatch {
                backup_id,
                sequence,
                state: state.to_string(),
            },
            RestoreRunnerError::Io(error) => Self::Io(error),
            RestoreRunnerError::Json(error) => Self::Json(error),
            RestoreRunnerError::Journal(error) => Self::RestoreApplyJournal(error),
        }
    }
}
