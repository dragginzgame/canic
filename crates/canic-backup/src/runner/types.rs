use crate::{
    execution::{BackupExecutionJournalOperation, BackupExecutionResumeSummary},
    persistence::CommandLifetimeHandle,
    plan::{BackupExecutionPreflightReceipts, BackupPlan},
};
use serde::Serialize;
use std::{error::Error as StdError, fmt, path::Path, path::PathBuf};
use thiserror::Error as ThisError;

use crate::persistence::JournalLockError;

///
/// BackupRunnerConfig
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRunnerConfig {
    pub out: PathBuf,
    pub max_steps: Option<usize>,
    pub updated_at: Option<String>,
    pub tool_name: String,
    pub tool_version: String,
}

///
/// BackupRunnerExecutor
///

pub trait BackupRunnerExecutor {
    /// Prove execution preflights before any mutating operation runs.
    fn preflight_receipts(
        &mut self,
        plan: &BackupPlan,
        preflight_id: &str,
        validated_at: &str,
        expires_at: &str,
    ) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError>;

    /// Observe one canister's authoritative lifecycle status.
    fn canister_status(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerCanisterStatus, BackupRunnerCommandError>;

    /// Stop one selected canister.
    fn stop_canister(
        &mut self,
        canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError>;

    /// Start one selected canister.
    fn start_canister(
        &mut self,
        canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError>;

    /// Create one selected canister snapshot and return the typed snapshot receipt.
    fn create_snapshot(
        &mut self,
        canister_id: &str,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<BackupRunnerSnapshotReceipt, BackupRunnerCommandError>;

    /// Download one selected snapshot into a temporary artifact directory.
    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
        command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError>;
}

///
/// BackupRunnerCanisterStatus
///
/// Typed lifecycle status used to reconcile interrupted backup operations.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupRunnerCanisterStatus {
    Running,
    Stopped,
    Stopping,
}

impl BackupRunnerCanisterStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Stopped => "Stopped",
            Self::Stopping => "Stopping",
        }
    }
}

///
/// BackupRunnerSnapshotReceipt
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRunnerSnapshotReceipt {
    pub snapshot_id: String,
    pub taken_at_timestamp: Option<u64>,
    pub total_size_bytes: Option<u64>,
}

///
/// BackupRunnerCommandError
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRunnerCommandError {
    pub status: String,
    pub message: String,
}

impl BackupRunnerCommandError {
    #[must_use]
    pub fn failed(status: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for BackupRunnerCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.status, self.message)
    }
}

impl StdError for BackupRunnerCommandError {}

///
/// BackupRunnerError
///

#[derive(Debug, ThisError)]
pub enum BackupRunnerError {
    #[error("backup execution journal is locked: {lock_path}")]
    JournalLocked { lock_path: String },

    #[error("backup execution journal lock path is unsafe: {lock_path} ({kind})")]
    JournalLockUnsafeEntry { lock_path: String, kind: String },

    #[error(
        "backup operation {sequence} {operation_id} has an external command still running: {lock_path}"
    )]
    CommandInFlight {
        sequence: usize,
        operation_id: String,
        lock_path: String,
    },

    #[error(
        "backup operation {sequence} {operation_id} has a quiescent command with an unknown external outcome: {lock_path}"
    )]
    CommandOutcomeUnknown {
        sequence: usize,
        operation_id: String,
        lock_path: String,
    },

    #[error(
        "backup operation {sequence} {operation_id} command lock path is unsafe: {lock_path} ({kind})"
    )]
    CommandLockUnsafeEntry {
        sequence: usize,
        operation_id: String,
        lock_path: String,
        kind: String,
    },

    #[error(
        "backup operation {sequence} {operation_id} observed unsettled canister status {status}"
    )]
    CanisterStatusUnsettled {
        sequence: usize,
        operation_id: String,
        status: &'static str,
    },

    #[error("backup operation {sequence} canister status observation failed: {status}: {message}")]
    CanisterStatusFailed {
        sequence: usize,
        status: String,
        message: String,
    },

    #[error("backup operation {sequence} {operation_id} is missing its command lifetime handle")]
    MissingCommandLifetime {
        sequence: usize,
        operation_id: String,
    },

    #[error(
        "download journal backup id does not match the backup plan: expected={expected}, actual={actual}"
    )]
    DownloadJournalBackupIdMismatch { expected: String, actual: String },

    #[error(
        "download journal topology receipt {field} does not match the backup plan: expected={expected}, actual={actual}"
    )]
    DownloadJournalTopologyMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },

    #[error("backup operation {sequence} has no target canister")]
    MissingOperationTarget { sequence: usize },

    #[error("backup operation {sequence} has no snapshot id for target {target_canister_id}")]
    MissingSnapshotId {
        sequence: usize,
        target_canister_id: String,
    },

    #[error(
        "backup operation {sequence} has no artifact journal entry for target {target_canister_id}"
    )]
    MissingArtifactEntry {
        sequence: usize,
        target_canister_id: String,
    },

    #[error(
        "backup operation {sequence} artifact temp path for target {target_canister_id} does not match expected runner path: journal={journal_path}, expected={expected_path}"
    )]
    ArtifactTempPathMismatch {
        sequence: usize,
        target_canister_id: String,
        journal_path: String,
        expected_path: String,
    },

    #[error("backup operation {sequence} failed: {status}: {message}")]
    CommandFailed {
        sequence: usize,
        status: String,
        message: String,
    },

    #[error("backup preflight failed: {status}: {message}")]
    PreflightFailed { status: String, message: String },

    #[error("backup execution has no operation ready to run")]
    NoReadyOperation,

    #[error("backup execution is blocked: {reasons:?}")]
    Blocked { reasons: Vec<String> },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] crate::persistence::PersistenceError),

    #[error(transparent)]
    BackupPlan(#[from] crate::plan::BackupPlanError),

    #[error(transparent)]
    ExecutionJournal(#[from] crate::execution::BackupExecutionJournalError),

    #[error(transparent)]
    Journal(#[from] crate::journal::JournalValidationError),

    #[error(transparent)]
    Checksum(#[from] crate::artifacts::ArtifactChecksumError),

    #[error(transparent)]
    Manifest(#[from] crate::manifest::ManifestValidationError),
}

impl From<JournalLockError> for BackupRunnerError {
    fn from(error: JournalLockError) -> Self {
        match error {
            JournalLockError::Locked { lock_path } => Self::JournalLocked { lock_path },
            JournalLockError::UnsafeEntry { lock_path, kind } => {
                Self::JournalLockUnsafeEntry { lock_path, kind }
            }
            JournalLockError::Io(error) => Self::Io(error),
        }
    }
}

///
/// BackupRunResponse
///

#[derive(Clone, Debug, Serialize)]
pub struct BackupRunResponse {
    pub run_id: String,
    pub plan_id: String,
    pub backup_id: String,
    pub complete: bool,
    pub max_steps_reached: bool,
    pub executed_operation_count: usize,
    pub executed_operations: Vec<BackupRunExecutedOperation>,
    pub execution: BackupExecutionResumeSummary,
}

///
/// BackupRunExecutedOperation
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupRunExecutedOperation {
    pub sequence: usize,
    pub operation_id: String,
    pub kind: String,
    pub target_canister_id: Option<String>,
    pub outcome: String,
}

impl BackupRunExecutedOperation {
    pub(super) fn completed(operation: &BackupExecutionJournalOperation) -> Self {
        Self::from_operation(operation, "completed")
    }

    pub(super) fn failed(operation: &BackupExecutionJournalOperation) -> Self {
        Self::from_operation(operation, "failed")
    }

    fn from_operation(operation: &BackupExecutionJournalOperation, outcome: &str) -> Self {
        Self {
            sequence: operation.sequence,
            operation_id: operation.operation_id.clone(),
            kind: format!("{:?}", operation.kind),
            target_canister_id: operation.target_canister_id.clone(),
            outcome: outcome.to_string(),
        }
    }
}
