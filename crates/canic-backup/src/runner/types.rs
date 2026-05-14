use crate::{
    execution::{BackupExecutionJournalOperation, BackupExecutionResumeSummary},
    plan::{BackupExecutionPreflightReceipts, BackupPlan},
};
use serde::Serialize;
use std::{error::Error as StdError, fmt, path::Path, path::PathBuf};
use thiserror::Error as ThisError;

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

    /// Stop one selected canister.
    fn stop_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError>;

    /// Start one selected canister.
    fn start_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError>;

    /// Create one selected canister snapshot and return the typed snapshot receipt.
    fn create_snapshot(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerSnapshotReceipt, BackupRunnerCommandError>;

    /// Download one selected snapshot into a temporary artifact directory.
    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), BackupRunnerCommandError>;
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
