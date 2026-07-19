use crate::backup::BackupCommandError;
use canic_backup::{
    artifacts::ArtifactChecksumError,
    persistence::PersistenceError,
    restore::{
        RestoreApplyDryRunError, RestoreApplyJournalError, RestorePersistenceError,
        RestorePlanError, RestoreRunnerError,
    },
};
use canic_host::icp::IcpCommandError;
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

    #[error("--require-verified requires a backup reference or --backup-dir")]
    RequireVerifiedNeedsBackupDir,

    #[error(
        "restore backup reference {backup_ref} has no prepared plan at {path}; run `canic restore prepare {backup_ref}` first"
    )]
    PreparedPlanMissing { backup_ref: String, path: String },

    #[error(
        "restore backup reference {backup_ref} has no prepared apply journal at {path}; run `canic restore prepare {backup_ref}` first"
    )]
    PreparedJournalMissing { backup_ref: String, path: String },

    #[error(
        "restore backup reference {backup_ref} prepared journal at {path} has no backup_root; run `canic restore prepare {backup_ref}` again"
    )]
    PreparedJournalBackupRootMissing { backup_ref: String, path: String },

    #[error(
        "restore backup reference {backup_ref} prepared journal at {path} points at backup_root {actual}, expected {expected}; run `canic restore prepare {backup_ref}` again"
    )]
    PreparedJournalBackupRootMismatch {
        backup_ref: String,
        path: String,
        expected: String,
        actual: String,
    },

    #[error("restore run command failed for operation {sequence}: status={status}")]
    RestoreRunCommandFailed { sequence: usize, status: String },

    #[error("restore apply journal is locked: {lock_path}")]
    RestoreApplyJournalLocked { lock_path: String },

    #[error("restore apply journal lock path is unsafe: {lock_path} ({kind})")]
    RestoreApplyJournalLockUnsafeEntry { lock_path: String, kind: String },

    #[error(
        "restore operation {sequence} {operation:?} has an external command still running: {lock_path}"
    )]
    RestoreCommandInFlight {
        sequence: usize,
        operation: canic_backup::restore::RestoreApplyOperationKind,
        lock_path: String,
    },

    #[error(
        "restore operation {sequence} {operation:?} has a quiescent command with an unknown external outcome: {lock_path}"
    )]
    RestoreCommandOutcomeUnknown {
        sequence: usize,
        operation: canic_backup::restore::RestoreApplyOperationKind,
        lock_path: String,
    },

    #[error(
        "restore operation {sequence} {operation:?} command lock path is unsafe: {lock_path} ({kind})"
    )]
    RestoreCommandLockUnsafeEntry {
        sequence: usize,
        operation: canic_backup::restore::RestoreApplyOperationKind,
        lock_path: String,
        kind: String,
    },

    #[error("restore plan for backup {backup_id} is not restore-ready: reasons={reasons:?}")]
    RestoreNotReady {
        backup_id: String,
        reasons: Vec<String>,
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

    #[error("restore artifact staging checksum failed for operation {sequence}")]
    RestoreArtifactStageChecksum {
        sequence: usize,
        #[source]
        source: ArtifactChecksumError,
    },

    #[error("restore artifact staging IO failed at {path}")]
    RestoreArtifactStageIo {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("restore artifact staging operation {sequence} is missing {field}")]
    RestoreArtifactStageMissingField {
        sequence: usize,
        field: &'static str,
    },

    #[error("restore artifact staging path is unsafe or occupied: {path}")]
    RestoreArtifactStagePathConflict { path: std::path::PathBuf },

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
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    Backup(#[from] BackupCommandError),

    #[error(transparent)]
    RestorePlan(#[from] RestorePlanError),

    #[error(transparent)]
    RestoreApplyDryRun(#[from] RestoreApplyDryRunError),

    #[error(transparent)]
    RestoreApplyJournal(#[from] RestoreApplyJournalError),

    #[error(transparent)]
    RestorePersistence(#[from] RestorePersistenceError),
}

impl From<RestoreRunnerError> for RestoreCommandError {
    fn from(error: RestoreRunnerError) -> Self {
        match error {
            RestoreRunnerError::ArtifactStageChecksum { sequence, source } => {
                Self::RestoreArtifactStageChecksum { sequence, source }
            }
            RestoreRunnerError::ArtifactStageIo { path, source } => {
                Self::RestoreArtifactStageIo { path, source }
            }
            RestoreRunnerError::ArtifactStageMissingField { sequence, field } => {
                Self::RestoreArtifactStageMissingField { sequence, field }
            }
            RestoreRunnerError::ArtifactStagePathConflict { path } => {
                Self::RestoreArtifactStagePathConflict { path }
            }
            RestoreRunnerError::CommandFailed { sequence, status } => {
                Self::RestoreRunCommandFailed { sequence, status }
            }
            RestoreRunnerError::JournalLocked { lock_path } => {
                Self::RestoreApplyJournalLocked { lock_path }
            }
            RestoreRunnerError::JournalLockUnsafeEntry { lock_path, kind } => {
                Self::RestoreApplyJournalLockUnsafeEntry { lock_path, kind }
            }
            RestoreRunnerError::CommandInFlight {
                sequence,
                operation,
                lock_path,
            } => Self::RestoreCommandInFlight {
                sequence,
                operation,
                lock_path,
            },
            RestoreRunnerError::CommandOutcomeUnknown {
                sequence,
                operation,
                lock_path,
            } => Self::RestoreCommandOutcomeUnknown {
                sequence,
                operation,
                lock_path,
            },
            RestoreRunnerError::CommandLockUnsafeEntry {
                sequence,
                operation,
                lock_path,
                kind,
            } => Self::RestoreCommandLockUnsafeEntry {
                sequence,
                operation,
                lock_path,
                kind,
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
            RestoreRunnerError::Persistence(error) => Self::RestorePersistence(error),
        }
    }
}
