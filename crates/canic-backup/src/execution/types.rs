use crate::plan::BackupOperationKind;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// BackupExecutionJournal
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupExecutionJournal {
    pub journal_version: u16,
    pub plan_id: String,
    pub run_id: String,
    pub preflight_id: Option<String>,
    pub preflight_accepted: bool,
    pub restart_required: bool,
    pub operations: Vec<BackupExecutionJournalOperation>,
    pub operation_receipts: Vec<BackupExecutionOperationReceipt>,
}

///
/// BackupExecutionJournalOperation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupExecutionJournalOperation {
    pub sequence: usize,
    pub operation_id: String,
    pub kind: BackupOperationKind,
    pub target_canister_id: Option<String>,
    pub state: BackupExecutionOperationState,
    pub state_updated_at: Option<String>,
    pub blocking_reasons: Vec<String>,
}

///
/// BackupExecutionOperationState
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupExecutionOperationState {
    Ready,
    Pending,
    Blocked,
    Completed,
    Failed,
    Skipped,
}

///
/// BackupExecutionOperationReceipt
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupExecutionOperationReceipt {
    pub plan_id: String,
    pub run_id: String,
    pub preflight_id: Option<String>,
    pub sequence: usize,
    pub operation_id: String,
    pub kind: BackupOperationKind,
    pub target_canister_id: Option<String>,
    pub outcome: BackupExecutionOperationReceiptOutcome,
    pub updated_at: Option<String>,
    pub snapshot_id: Option<String>,
    #[serde(default)]
    pub snapshot_taken_at_timestamp: Option<u64>,
    #[serde(default)]
    pub snapshot_total_size_bytes: Option<u64>,
    pub artifact_path: Option<String>,
    pub checksum: Option<String>,
    pub failure_reason: Option<String>,
}

///
/// BackupExecutionOperationReceiptOutcome
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupExecutionOperationReceiptOutcome {
    Completed,
    Failed,
    Skipped,
}

///
/// BackupExecutionResumeSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupExecutionResumeSummary {
    pub plan_id: String,
    pub run_id: String,
    pub preflight_id: Option<String>,
    pub preflight_accepted: bool,
    pub restart_required: bool,
    pub total_operations: usize,
    pub ready_operations: usize,
    pub pending_operations: usize,
    pub blocked_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub skipped_operations: usize,
    pub next_operation: Option<BackupExecutionJournalOperation>,
}

///
/// BackupExecutionJournalError
///

#[derive(Debug, ThisError)]
pub enum BackupExecutionJournalError {
    #[error("invalid backup plan for execution journal: {0}")]
    InvalidPlan(String),

    #[error("unsupported backup execution journal version {0}")]
    UnsupportedVersion(u16),

    #[error("backup execution journal field {0} is required")]
    MissingField(&'static str),

    #[error("backup execution journal has duplicate operation sequence {0}")]
    DuplicateSequence(usize),

    #[error("backup execution journal is missing operation sequence {0}")]
    MissingSequence(usize),

    #[error("accepted preflight is missing preflight_id")]
    AcceptedPreflightMissingId,

    #[error("restart_required does not match execution operation state")]
    RestartRequiredMismatch,

    #[error("preflight already accepted as {existing}, cannot accept {attempted}")]
    PreflightAlreadyAccepted { existing: String, attempted: String },

    #[error("preflight receipt plan id {actual} does not match execution journal plan {expected}")]
    PreflightPlanMismatch { expected: String, actual: String },

    #[error("mutating operation {sequence} is ready before preflight acceptance")]
    MutationReadyBeforePreflight { sequence: usize },

    #[error("mutating operation {sequence} cannot run before preflight acceptance")]
    MutationBeforePreflightAccepted { sequence: usize },

    #[error("operation {0} is missing a blocking or failure reason")]
    OperationMissingReason(usize),

    #[error("operation {0} cannot have blocking reasons in its current state")]
    UnblockedOperationHasReasons(usize),

    #[error("operation {0} was not found")]
    OperationNotFound(usize),

    #[error("operation {sequence} cannot transition from {from:?} to {to:?}")]
    InvalidOperationTransition {
        sequence: usize,
        from: BackupExecutionOperationState,
        to: BackupExecutionOperationState,
    },

    #[error("operation {requested} cannot advance before operation {next}")]
    OutOfOrderOperationTransition { requested: usize, next: usize },

    #[error("no operation can be advanced")]
    NoTransitionableOperation,

    #[error("operation {0} is not failed")]
    OperationNotFailed(usize),

    #[error("operation receipt references missing operation {0}")]
    ReceiptOperationNotFound(usize),

    #[error("operation receipt does not match operation {sequence}")]
    ReceiptOperationMismatch { sequence: usize },

    #[error("operation receipt does not match journal {sequence}")]
    ReceiptJournalMismatch { sequence: usize },

    #[error("operation receipt does not match accepted preflight {sequence}")]
    ReceiptPreflightMismatch { sequence: usize },

    #[error("operation receipt {sequence} has no pending operation")]
    ReceiptWithoutPendingOperation { sequence: usize },
}
