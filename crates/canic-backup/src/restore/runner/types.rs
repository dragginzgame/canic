use super::{
    RestoreApplyCommandConfig, RestoreApplyCommandOutputPair, RestoreApplyJournalError,
    RestoreApplyJournalOperation, RestoreApplyJournalReport, RestoreApplyOperationKind,
    RestoreApplyOperationKindCounts, RestoreApplyPendingSummary, RestoreApplyProgressSummary,
    RestoreApplyReportOperation, RestoreApplyReportOutcome, RestoreApplyRunnerCommand,
    constants::*,
};
use serde::Serialize;
use std::{io, path::PathBuf};
use thiserror::Error as ThisError;

///
/// RestoreRunnerConfig
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreRunnerConfig {
    pub journal: PathBuf,
    pub command: RestoreApplyCommandConfig,
    pub max_steps: Option<usize>,
    pub updated_at: Option<String>,
}

///
/// RestoreRunnerCommandExecutor
///

pub trait RestoreRunnerCommandExecutor {
    /// Execute one rendered restore runner command.
    fn execute(
        &mut self,
        command: &RestoreApplyRunnerCommand,
    ) -> Result<RestoreRunnerCommandOutput, io::Error>;
}

///
/// RestoreRunnerCommandOutput
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreRunnerCommandOutput {
    pub success: bool,
    pub status: String,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

///
/// RestoreRunnerError
///

#[derive(Debug, ThisError)]
pub enum RestoreRunnerError {
    #[error("restore run command failed for operation {sequence}: status={status}")]
    CommandFailed { sequence: usize, status: String },

    #[error("restore apply journal is locked: {lock_path}")]
    JournalLocked { lock_path: String },

    #[error(
        "restore apply journal for backup {backup_id} has pending operations: pending={pending_operations}, next={next_transition_sequence:?}"
    )]
    Pending {
        backup_id: String,
        pending_operations: usize,
        next_transition_sequence: Option<usize>,
    },

    #[error(
        "restore apply journal for backup {backup_id} has failed operations: failed={failed_operations}"
    )]
    Failed {
        backup_id: String,
        failed_operations: usize,
    },

    #[error("restore apply journal for backup {backup_id} is not ready: reasons={reasons:?}")]
    NotReady {
        backup_id: String,
        reasons: Vec<String>,
    },

    #[error(
        "restore apply journal for backup {backup_id} has no executable command: operation_available={operation_available}, complete={complete}, blocked_reasons={blocked_reasons:?}"
    )]
    CommandUnavailable {
        backup_id: String,
        operation_available: bool,
        complete: bool,
        blocked_reasons: Vec<String>,
    },

    #[error(
        "restore apply journal next operation changed before claim: expected={expected}, actual={actual:?}"
    )]
    ClaimSequenceMismatch {
        expected: usize,
        actual: Option<usize>,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Journal(#[from] RestoreApplyJournalError),
}

///
/// RestoreRunResponse
///

#[derive(Clone, Debug, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Runner response exposes stable JSON status flags for operators and CI"
)]
pub struct RestoreRunResponse {
    pub run_version: u16,
    pub backup_id: String,
    pub run_mode: &'static str,
    pub dry_run: bool,
    pub execute: bool,
    pub unclaim_pending: bool,
    pub stopped_reason: &'static str,
    pub next_action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_state_updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_steps_reached: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub executed_operations: Vec<RestoreRunExecutedOperation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operation_receipts: Vec<RestoreRunOperationReceipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_receipt_count: Option<usize>,
    pub operation_receipt_summary: RestoreRunReceiptSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_operation_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovered_operation: Option<RestoreApplyJournalOperation>,
    pub ready: bool,
    pub complete: bool,
    pub attention_required: bool,
    pub outcome: RestoreApplyReportOutcome,
    pub operation_count: usize,
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub progress: RestoreApplyProgressSummary,
    pub pending_summary: RestoreApplyPendingSummary,
    pub pending_operations: usize,
    pub ready_operations: usize,
    pub blocked_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub blocked_reasons: Vec<String>,
    pub next_transition: Option<RestoreApplyReportOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<RestoreApplyRunnerCommand>,
}

impl RestoreRunResponse {
    // Build the shared native runner response fields from an apply journal report.
    pub(super) fn from_report(
        backup_id: String,
        report: RestoreApplyJournalReport,
        mode: RestoreRunResponseMode,
    ) -> Self {
        Self {
            run_version: RESTORE_RUN_RESPONSE_VERSION,
            backup_id,
            run_mode: mode.run_mode,
            dry_run: mode.dry_run,
            execute: mode.execute,
            unclaim_pending: mode.unclaim_pending,
            stopped_reason: mode.stopped_reason,
            next_action: mode.next_action,
            requested_state_updated_at: None,
            max_steps_reached: None,
            executed_operations: Vec::new(),
            operation_receipts: Vec::new(),
            operation_receipt_count: Some(0),
            operation_receipt_summary: RestoreRunReceiptSummary::default(),
            executed_operation_count: None,
            recovered_operation: None,
            ready: report.ready,
            complete: report.complete,
            attention_required: report.attention_required,
            outcome: report.outcome,
            operation_count: report.operation_count,
            operation_counts: report.operation_counts,
            progress: report.progress,
            pending_summary: report.pending_summary,
            pending_operations: report.pending_operations,
            ready_operations: report.ready_operations,
            blocked_operations: report.blocked_operations,
            completed_operations: report.completed_operations,
            failed_operations: report.failed_operations,
            blocked_reasons: report.blocked_reasons,
            next_transition: report.next_transition,
            operation_available: None,
            command_available: None,
            command: None,
        }
    }

    // Replace the detailed receipt stream and refresh the compact counters.
    pub(super) fn set_operation_receipts(&mut self, receipts: Vec<RestoreRunOperationReceipt>) {
        self.operation_receipt_summary = RestoreRunReceiptSummary::from_receipts(&receipts);
        self.operation_receipt_count = Some(receipts.len());
        self.operation_receipts = receipts;
    }

    // Echo the caller-provided state marker for receipt-free runner summaries.
    pub(super) fn set_requested_state_updated_at(&mut self, updated_at: Option<&String>) {
        self.requested_state_updated_at = updated_at.cloned();
    }
}

///
/// RestoreRunReceiptSummary
///

#[derive(Clone, Debug, Default, Serialize)]
pub struct RestoreRunReceiptSummary {
    pub total_receipts: usize,
    pub command_completed: usize,
    pub command_failed: usize,
    pub pending_recovered: usize,
}

impl RestoreRunReceiptSummary {
    // Count restore runner receipt classes for script-friendly summaries.
    fn from_receipts(receipts: &[RestoreRunOperationReceipt]) -> Self {
        let mut summary = Self {
            total_receipts: receipts.len(),
            ..Self::default()
        };

        for receipt in receipts {
            match receipt.event {
                RESTORE_RUN_RECEIPT_COMPLETED => summary.command_completed += 1,
                RESTORE_RUN_RECEIPT_FAILED => summary.command_failed += 1,
                RESTORE_RUN_RECEIPT_RECOVERED_PENDING => summary.pending_recovered += 1,
                _ => {}
            }
        }

        summary
    }
}

///
/// RestoreRunOperationReceipt
///

#[derive(Clone, Debug, Serialize)]
pub struct RestoreRunOperationReceipt {
    pub event: &'static str,
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
    pub target_canister: String,
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<RestoreApplyRunnerCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

impl RestoreRunOperationReceipt {
    // Build a receipt for a completed runner command.
    pub(super) fn completed(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        updated_at: Option<String>,
    ) -> Self {
        Self::from_operation(
            RESTORE_RUN_RECEIPT_COMPLETED,
            operation,
            RESTORE_RUN_EXECUTED_COMPLETED,
            updated_at,
            Some(command),
            Some(status),
        )
    }

    // Build a receipt for a failed runner command.
    pub(super) fn failed(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        updated_at: Option<String>,
    ) -> Self {
        Self::from_operation(
            RESTORE_RUN_RECEIPT_FAILED,
            operation,
            RESTORE_RUN_EXECUTED_FAILED,
            updated_at,
            Some(command),
            Some(status),
        )
    }

    // Build a receipt for a recovered pending operation.
    pub(super) fn recovered_pending(
        operation: RestoreApplyJournalOperation,
        updated_at: Option<String>,
    ) -> Self {
        Self::from_operation(
            RESTORE_RUN_RECEIPT_RECOVERED_PENDING,
            operation,
            RESTORE_RUN_RECEIPT_STATE_READY,
            updated_at,
            None,
            None,
        )
    }

    // Map one operation event into a compact audit receipt.
    fn from_operation(
        event: &'static str,
        operation: RestoreApplyJournalOperation,
        state: &'static str,
        updated_at: Option<String>,
        command: Option<RestoreApplyRunnerCommand>,
        status: Option<String>,
    ) -> Self {
        Self {
            event,
            sequence: operation.sequence,
            operation: operation.operation,
            target_canister: operation.target_canister,
            state,
            updated_at,
            command,
            status,
        }
    }
}

///
/// RestoreRunExecutedOperation
///

#[derive(Clone, Debug, Serialize)]
pub struct RestoreRunExecutedOperation {
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
    pub target_canister: String,
    pub command: RestoreApplyRunnerCommand,
    pub status: String,
    pub state: &'static str,
}

impl RestoreRunExecutedOperation {
    // Build a completed executed-operation summary row from a runner operation.
    pub(super) fn completed(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
    ) -> Self {
        Self::from_operation(operation, command, status, RESTORE_RUN_EXECUTED_COMPLETED)
    }

    // Build a failed executed-operation summary row from a runner operation.
    pub(super) fn failed(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
    ) -> Self {
        Self::from_operation(operation, command, status, RESTORE_RUN_EXECUTED_FAILED)
    }

    // Map a journal operation into the compact runner execution row.
    fn from_operation(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        state: &'static str,
    ) -> Self {
        Self {
            sequence: operation.sequence,
            operation: operation.operation,
            target_canister: operation.target_canister,
            command,
            status,
            state,
        }
    }
}

///
/// RestoreRunnerOutcome
///

pub struct RestoreRunnerOutcome {
    pub response: RestoreRunResponse,
    pub error: Option<RestoreRunnerError>,
}

impl RestoreRunnerOutcome {
    // Build a successful runner response with no deferred error.
    pub(super) const fn ok(response: RestoreRunResponse) -> Self {
        Self {
            response,
            error: None,
        }
    }
}

///
/// RestoreStoppedPreconditionFailure
///

pub(super) struct RestoreStoppedPreconditionFailure {
    pub(super) command: RestoreApplyRunnerCommand,
    pub(super) status_label: String,
    pub(super) output: RestoreApplyCommandOutputPair,
    pub(super) failure_reason: String,
}

///
/// RestoreRunResponseMode
///

pub(super) struct RestoreRunResponseMode {
    run_mode: &'static str,
    dry_run: bool,
    execute: bool,
    unclaim_pending: bool,
    stopped_reason: &'static str,
    next_action: &'static str,
}

impl RestoreRunResponseMode {
    // Build a response mode from the stable JSON mode flags and action labels.
    const fn new(
        run_mode: &'static str,
        dry_run: bool,
        execute: bool,
        unclaim_pending: bool,
        stopped_reason: &'static str,
        next_action: &'static str,
    ) -> Self {
        Self {
            run_mode,
            dry_run,
            execute,
            unclaim_pending,
            stopped_reason,
            next_action,
        }
    }

    // Build a dry-run response mode with a computed stop reason and action.
    pub(super) const fn dry_run(stopped_reason: &'static str, next_action: &'static str) -> Self {
        Self::new(
            RESTORE_RUN_MODE_DRY_RUN,
            true,
            false,
            false,
            stopped_reason,
            next_action,
        )
    }

    // Build an execute response mode with a computed stop reason and action.
    pub(super) const fn execute(stopped_reason: &'static str, next_action: &'static str) -> Self {
        Self::new(
            RESTORE_RUN_MODE_EXECUTE,
            false,
            true,
            false,
            stopped_reason,
            next_action,
        )
    }

    // Build the pending-operation recovery response mode.
    pub(super) const fn unclaim_pending(next_action: &'static str) -> Self {
        Self::new(
            RESTORE_RUN_MODE_UNCLAIM_PENDING,
            false,
            false,
            true,
            RESTORE_RUN_STOPPED_RECOVERED_PENDING,
            next_action,
        )
    }
}

///
/// RestoreRunPreparedOperation
///

pub(super) struct RestoreRunPreparedOperation {
    pub(super) operation: RestoreApplyJournalOperation,
    pub(super) command: RestoreApplyRunnerCommand,
    pub(super) sequence: usize,
    pub(super) attempt: usize,
}

///
/// RestoreRunStepOutcome
///

pub(super) enum RestoreRunStepOutcome {
    Completed {
        executed_operation: RestoreRunExecutedOperation,
        operation_receipt: RestoreRunOperationReceipt,
    },
    Failed {
        executed_operation: RestoreRunExecutedOperation,
        operation_receipt: RestoreRunOperationReceipt,
        status: String,
    },
}
