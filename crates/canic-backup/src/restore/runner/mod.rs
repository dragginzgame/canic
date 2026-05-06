use super::{
    RestoreApplyCommandConfig, RestoreApplyCommandOutputPair, RestoreApplyCommandPreview,
    RestoreApplyJournal, RestoreApplyJournalError, RestoreApplyJournalOperation,
    RestoreApplyJournalReport, RestoreApplyOperationKind, RestoreApplyOperationKindCounts,
    RestoreApplyOperationReceipt, RestoreApplyOperationState, RestoreApplyPendingSummary,
    RestoreApplyProgressSummary, RestoreApplyReportOperation, RestoreApplyReportOutcome,
    RestoreApplyRunnerCommand,
};
use serde::Serialize;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};
use thiserror::Error as ThisError;

const RESTORE_RUN_MODE_DRY_RUN: &str = "dry-run";
const RESTORE_RUN_MODE_EXECUTE: &str = "execute";
const RESTORE_RUN_MODE_UNCLAIM_PENDING: &str = "unclaim-pending";

const RESTORE_RUN_STOPPED_BLOCKED: &str = "blocked";
const RESTORE_RUN_STOPPED_COMMAND_FAILED: &str = "command-failed";
const RESTORE_RUN_STOPPED_COMPLETE: &str = "complete";
const RESTORE_RUN_STOPPED_MAX_STEPS: &str = "max-steps-reached";
const RESTORE_RUN_STOPPED_PENDING: &str = "pending";
const RESTORE_RUN_STOPPED_PREVIEW: &str = "preview";
const RESTORE_RUN_STOPPED_READY: &str = "ready";
const RESTORE_RUN_STOPPED_RECOVERED_PENDING: &str = "recovered-pending";

const RESTORE_RUN_ACTION_DONE: &str = "done";
const RESTORE_RUN_ACTION_FIX_BLOCKED: &str = "fix-blocked-journal";
const RESTORE_RUN_ACTION_INSPECT_FAILED: &str = "inspect-failed-operation";
const RESTORE_RUN_ACTION_RERUN: &str = "rerun";
const RESTORE_RUN_ACTION_UNCLAIM_PENDING: &str = "unclaim-pending";

pub const RESTORE_RUN_RECEIPT_COMPLETED: &str = "command-completed";
pub const RESTORE_RUN_RECEIPT_FAILED: &str = "command-failed";
pub const RESTORE_RUN_RECEIPT_RECOVERED_PENDING: &str = "pending-recovered";

const RESTORE_RUN_EXECUTED_COMPLETED: &str = "completed";
const RESTORE_RUN_EXECUTED_FAILED: &str = "failed";
const RESTORE_RUN_RECEIPT_STATE_READY: &str = "ready";
const RESTORE_RUN_COMMAND_EXIT_PREFIX: &str = "runner-command-exit";
const RESTORE_RUN_RESPONSE_VERSION: u16 = 1;
const RESTORE_RUN_OUTPUT_RECEIPT_LIMIT: usize = 64 * 1024;

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
    pub batch_summary: RestoreRunBatchSummary,
    pub ready: bool,
    pub complete: bool,
    pub attention_required: bool,
    pub outcome: RestoreApplyReportOutcome,
    pub operation_count: usize,
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub operation_counts_supplied: bool,
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
    fn from_report(
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
            batch_summary: RestoreRunBatchSummary::from_counts(
                RestoreRunBatchStart::new(
                    None,
                    report.ready_operations,
                    report.progress.remaining_operations,
                ),
                0,
                report.ready_operations,
                report.progress.remaining_operations,
                false,
                report.complete,
            ),
            ready: report.ready,
            complete: report.complete,
            attention_required: report.attention_required,
            outcome: report.outcome,
            operation_count: report.operation_count,
            operation_counts: report.operation_counts,
            operation_counts_supplied: report.operation_counts_supplied,
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
    fn set_operation_receipts(&mut self, receipts: Vec<RestoreRunOperationReceipt>) {
        self.operation_receipt_summary = RestoreRunReceiptSummary::from_receipts(&receipts);
        self.operation_receipt_count = Some(receipts.len());
        self.operation_receipts = receipts;
    }

    // Echo the caller-provided state marker for receipt-free runner summaries.
    fn set_requested_state_updated_at(&mut self, updated_at: Option<&String>) {
        self.requested_state_updated_at = updated_at.cloned();
    }

    // Refresh batch counters after a dry-run, recovery, or execute pass.
    const fn set_batch_summary(
        &mut self,
        batch_start: RestoreRunBatchStart,
        executed_operations: usize,
        stopped_by_max_steps: bool,
    ) {
        self.batch_summary = RestoreRunBatchSummary::from_counts(
            batch_start,
            executed_operations,
            self.ready_operations,
            self.progress.remaining_operations,
            stopped_by_max_steps,
            self.complete,
        );
    }
}

///
/// RestoreRunBatchSummary
///

#[derive(Clone, Debug, Serialize)]
pub struct RestoreRunBatchSummary {
    pub requested_max_steps: Option<usize>,
    pub initial_ready_operations: usize,
    pub initial_remaining_operations: usize,
    pub executed_operations: usize,
    pub remaining_ready_operations: usize,
    pub remaining_operations: usize,
    pub ready_operations_delta: isize,
    pub remaining_operations_delta: isize,
    pub stopped_by_max_steps: bool,
    pub complete: bool,
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

///
/// RestoreRunnerOutcome
///

pub struct RestoreRunnerOutcome {
    pub response: RestoreRunResponse,
    pub error: Option<RestoreRunnerError>,
}

impl RestoreRunnerOutcome {
    // Build a successful runner response with no deferred error.
    const fn ok(response: RestoreRunResponse) -> Self {
        Self {
            response,
            error: None,
        }
    }
}

///
/// RestoreRunBatchStart
///

#[derive(Clone, Copy, Debug)]
struct RestoreRunBatchStart {
    requested_max_steps: Option<usize>,
    initial_ready_operations: usize,
    initial_remaining_operations: usize,
}

impl RestoreRunBatchStart {
    // Capture the runner counters observed before a dry-run, recovery, or execute pass.
    const fn new(
        requested_max_steps: Option<usize>,
        initial_ready_operations: usize,
        initial_remaining_operations: usize,
    ) -> Self {
        Self {
            requested_max_steps,
            initial_ready_operations,
            initial_remaining_operations,
        }
    }
}

impl RestoreRunBatchSummary {
    // Build the compact batch counters shown in every native runner response.
    const fn from_counts(
        batch_start: RestoreRunBatchStart,
        executed_operations: usize,
        remaining_ready_operations: usize,
        remaining_operations: usize,
        stopped_by_max_steps: bool,
        complete: bool,
    ) -> Self {
        Self {
            requested_max_steps: batch_start.requested_max_steps,
            initial_ready_operations: batch_start.initial_ready_operations,
            initial_remaining_operations: batch_start.initial_remaining_operations,
            executed_operations,
            remaining_ready_operations,
            remaining_operations,
            ready_operations_delta: remaining_ready_operations.cast_signed()
                - batch_start.initial_ready_operations.cast_signed(),
            remaining_operations_delta: remaining_operations.cast_signed()
                - batch_start.initial_remaining_operations.cast_signed(),
            stopped_by_max_steps,
            complete,
        }
    }
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

impl RestoreRunOperationReceipt {
    // Build a receipt for a completed runner command.
    fn completed(
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
    fn failed(
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
    fn recovered_pending(
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

impl RestoreRunExecutedOperation {
    // Build a completed executed-operation summary row from a runner operation.
    fn completed(
        operation: RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
    ) -> Self {
        Self::from_operation(operation, command, status, RESTORE_RUN_EXECUTED_COMPLETED)
    }

    // Build a failed executed-operation summary row from a runner operation.
    fn failed(
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
/// RestoreRunResponseMode
///

struct RestoreRunResponseMode {
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
    const fn dry_run(stopped_reason: &'static str, next_action: &'static str) -> Self {
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
    const fn execute(stopped_reason: &'static str, next_action: &'static str) -> Self {
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
    const fn unclaim_pending(next_action: &'static str) -> Self {
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

/// Build a no-mutation native restore runner preview from a journal file.
pub fn restore_run_dry_run(
    config: &RestoreRunnerConfig,
) -> Result<RestoreRunResponse, RestoreRunnerError> {
    let journal = read_apply_journal_file(&config.journal)?;
    let report = journal.report();
    let initial_ready_operations = report.ready_operations;
    let initial_remaining_operations = report.progress.remaining_operations;
    let preview = journal.next_command_preview_with_config(&config.command);
    let stopped_reason = restore_run_stopped_reason(&report, false, false);
    let next_action = restore_run_next_action(&report, false);

    let mut response = RestoreRunResponse::from_report(
        journal.backup_id,
        report,
        RestoreRunResponseMode::dry_run(stopped_reason, next_action),
    );
    response.set_requested_state_updated_at(config.updated_at.as_ref());
    response.set_batch_summary(
        RestoreRunBatchStart::new(
            config.max_steps,
            initial_ready_operations,
            initial_remaining_operations,
        ),
        0,
        false,
    );
    response.operation_available = Some(preview.operation_available);
    response.command_available = Some(preview.command_available);
    response.command = preview.command;
    Ok(response)
}

/// Recover an interrupted restore runner by unclaiming the pending operation.
pub fn restore_run_unclaim_pending(
    config: &RestoreRunnerConfig,
) -> Result<RestoreRunResponse, RestoreRunnerError> {
    let _lock = RestoreJournalLock::acquire(&config.journal)?;
    let mut journal = read_apply_journal_file(&config.journal)?;
    let initial_report = journal.report();
    let initial_ready_operations = initial_report.ready_operations;
    let initial_remaining_operations = initial_report.progress.remaining_operations;
    let recovered_operation = journal
        .next_transition_operation()
        .filter(|operation| operation.state == RestoreApplyOperationState::Pending)
        .cloned()
        .ok_or(RestoreApplyJournalError::NoPendingOperation)?;

    let recovered_updated_at = state_updated_at(config.updated_at.as_ref());
    journal.mark_next_operation_ready_at(Some(recovered_updated_at.clone()))?;
    write_apply_journal_file(&config.journal, &journal)?;

    let report = journal.report();
    let next_action = restore_run_next_action(&report, true);
    let mut response = RestoreRunResponse::from_report(
        journal.backup_id,
        report,
        RestoreRunResponseMode::unclaim_pending(next_action),
    );
    response.set_requested_state_updated_at(config.updated_at.as_ref());
    response.set_batch_summary(
        RestoreRunBatchStart::new(
            config.max_steps,
            initial_ready_operations,
            initial_remaining_operations,
        ),
        0,
        false,
    );
    response.set_operation_receipts(vec![RestoreRunOperationReceipt::recovered_pending(
        recovered_operation.clone(),
        Some(recovered_updated_at),
    )]);
    response.recovered_operation = Some(recovered_operation);
    Ok(response)
}

/// Execute ready restore apply journal operations through generated runner commands.
pub fn restore_run_execute(
    config: &RestoreRunnerConfig,
) -> Result<RestoreRunResponse, RestoreRunnerError> {
    let run = restore_run_execute_result(config)?;
    if let Some(error) = run.error {
        return Err(error);
    }

    Ok(run.response)
}

#[expect(
    clippy::too_many_lines,
    reason = "runner execution keeps claim, command, receipt, and journal commit steps together"
)]
// Execute ready restore apply operations and retain any deferred runner error.
pub fn restore_run_execute_result(
    config: &RestoreRunnerConfig,
) -> Result<RestoreRunnerOutcome, RestoreRunnerError> {
    let _lock = RestoreJournalLock::acquire(&config.journal)?;
    let mut journal = read_apply_journal_file(&config.journal)?;
    let initial_report = journal.report();
    let batch_start = RestoreRunBatchStart::new(
        config.max_steps,
        initial_report.ready_operations,
        initial_report.progress.remaining_operations,
    );
    let mut executed_operations = Vec::new();
    let mut operation_receipts = Vec::new();

    loop {
        let report = journal.report();
        let max_steps_reached =
            restore_run_max_steps_reached(config, executed_operations.len(), &report);
        if report.complete || max_steps_reached {
            return Ok(RestoreRunnerOutcome::ok(restore_run_execute_summary(
                &journal,
                executed_operations,
                operation_receipts,
                max_steps_reached,
                config.updated_at.as_ref(),
                batch_start,
            )));
        }

        enforce_restore_run_executable(&journal, &report)?;
        let preview = journal.next_command_preview_with_config(&config.command);
        enforce_restore_run_command_available(&preview)?;

        let operation = preview
            .operation
            .clone()
            .ok_or_else(|| restore_command_unavailable_error(&preview))?;
        let command = preview
            .command
            .clone()
            .ok_or_else(|| restore_command_unavailable_error(&preview))?;
        let sequence = operation.sequence;
        let attempt = journal
            .operation_receipts
            .iter()
            .filter(|receipt| receipt.sequence == sequence)
            .count()
            + 1;

        enforce_apply_claim_sequence(sequence, &journal)?;
        journal.mark_operation_pending_at(
            sequence,
            Some(state_updated_at(config.updated_at.as_ref())),
        )?;
        write_apply_journal_file(&config.journal, &journal)?;

        let output = ProcessCommand::new(&command.program)
            .args(&command.args)
            .output()?;
        let status_label = exit_status_label(output.status);
        let output_pair = RestoreApplyCommandOutputPair::from_bytes(
            &output.stdout,
            &output.stderr,
            RESTORE_RUN_OUTPUT_RECEIPT_LIMIT,
        );
        if output.status.success() {
            let uploaded_snapshot_id =
                parse_uploaded_snapshot_id(&String::from_utf8_lossy(&output.stdout));
            let completed_updated_at = state_updated_at(config.updated_at.as_ref());
            journal.mark_operation_completed_at(sequence, Some(completed_updated_at.clone()))?;
            if operation.operation != RestoreApplyOperationKind::UploadSnapshot
                || uploaded_snapshot_id.is_some()
            {
                journal.record_operation_receipt(
                    RestoreApplyOperationReceipt::command_completed(
                        &operation,
                        command.clone(),
                        status_label.clone(),
                        Some(completed_updated_at.clone()),
                        output_pair.clone(),
                        attempt,
                        uploaded_snapshot_id,
                    ),
                )?;
            }
            write_apply_journal_file(&config.journal, &journal)?;
            executed_operations.push(RestoreRunExecutedOperation::completed(
                operation.clone(),
                command.clone(),
                status_label.clone(),
            ));
            operation_receipts.push(RestoreRunOperationReceipt::completed(
                operation,
                command,
                status_label,
                Some(completed_updated_at),
            ));
            continue;
        }

        let failed_updated_at = state_updated_at(config.updated_at.as_ref());
        let failure_reason = format!("{RESTORE_RUN_COMMAND_EXIT_PREFIX}-{status_label}");
        journal.mark_operation_failed_at(
            sequence,
            failure_reason.clone(),
            Some(failed_updated_at.clone()),
        )?;
        journal.record_operation_receipt(RestoreApplyOperationReceipt::command_failed(
            &operation,
            command.clone(),
            status_label.clone(),
            Some(failed_updated_at.clone()),
            output_pair,
            attempt,
            failure_reason,
        ))?;
        write_apply_journal_file(&config.journal, &journal)?;
        executed_operations.push(RestoreRunExecutedOperation::failed(
            operation.clone(),
            command.clone(),
            status_label.clone(),
        ));
        operation_receipts.push(RestoreRunOperationReceipt::failed(
            operation,
            command,
            status_label.clone(),
            Some(failed_updated_at),
        ));
        let response = restore_run_execute_summary(
            &journal,
            executed_operations,
            operation_receipts,
            false,
            config.updated_at.as_ref(),
            batch_start,
        );
        return Ok(RestoreRunnerOutcome {
            response,
            error: Some(RestoreRunnerError::CommandFailed {
                sequence,
                status: status_label,
            }),
        });
    }
}

// Check whether execute mode has reached its requested operation batch size.
fn restore_run_max_steps_reached(
    config: &RestoreRunnerConfig,
    executed_operation_count: usize,
    report: &RestoreApplyJournalReport,
) -> bool {
    config.max_steps == Some(executed_operation_count) && !report.complete
}

// Build the final native runner execution summary.
fn restore_run_execute_summary(
    journal: &RestoreApplyJournal,
    executed_operations: Vec<RestoreRunExecutedOperation>,
    operation_receipts: Vec<RestoreRunOperationReceipt>,
    max_steps_reached: bool,
    requested_state_updated_at: Option<&String>,
    batch_start: RestoreRunBatchStart,
) -> RestoreRunResponse {
    let report = journal.report();
    let executed_operation_count = executed_operations.len();
    let stopped_reason = restore_run_stopped_reason(&report, max_steps_reached, true);
    let next_action = restore_run_next_action(&report, false);

    let mut response = RestoreRunResponse::from_report(
        journal.backup_id.clone(),
        report,
        RestoreRunResponseMode::execute(stopped_reason, next_action),
    );
    response.set_requested_state_updated_at(requested_state_updated_at);
    response.set_batch_summary(batch_start, executed_operation_count, max_steps_reached);
    response.max_steps_reached = Some(max_steps_reached);
    response.executed_operation_count = Some(executed_operation_count);
    response.executed_operations = executed_operations;
    response.set_operation_receipts(operation_receipts);
    response
}

// Classify why the native runner stopped for operator summaries.
const fn restore_run_stopped_reason(
    report: &RestoreApplyJournalReport,
    max_steps_reached: bool,
    executed: bool,
) -> &'static str {
    if report.complete {
        return RESTORE_RUN_STOPPED_COMPLETE;
    }
    if report.failed_operations > 0 {
        return RESTORE_RUN_STOPPED_COMMAND_FAILED;
    }
    if report.pending_operations > 0 {
        return RESTORE_RUN_STOPPED_PENDING;
    }
    if !report.ready || report.blocked_operations > 0 {
        return RESTORE_RUN_STOPPED_BLOCKED;
    }
    if max_steps_reached {
        return RESTORE_RUN_STOPPED_MAX_STEPS;
    }
    if executed {
        return RESTORE_RUN_STOPPED_READY;
    }
    RESTORE_RUN_STOPPED_PREVIEW
}

// Recommend the next operator action for the native runner summary.
const fn restore_run_next_action(
    report: &RestoreApplyJournalReport,
    recovered_pending: bool,
) -> &'static str {
    if report.complete {
        return RESTORE_RUN_ACTION_DONE;
    }
    if report.failed_operations > 0 {
        return RESTORE_RUN_ACTION_INSPECT_FAILED;
    }
    if report.pending_operations > 0 {
        return RESTORE_RUN_ACTION_UNCLAIM_PENDING;
    }
    if !report.ready || report.blocked_operations > 0 {
        return RESTORE_RUN_ACTION_FIX_BLOCKED;
    }
    if recovered_pending {
        return RESTORE_RUN_ACTION_RERUN;
    }
    RESTORE_RUN_ACTION_RERUN
}

// Ensure the journal can be advanced by the native restore runner.
fn enforce_restore_run_executable(
    journal: &RestoreApplyJournal,
    report: &RestoreApplyJournalReport,
) -> Result<(), RestoreRunnerError> {
    if report.pending_operations > 0 {
        return Err(RestoreRunnerError::Pending {
            backup_id: report.backup_id.clone(),
            pending_operations: report.pending_operations,
            next_transition_sequence: report
                .next_transition
                .as_ref()
                .map(|operation| operation.sequence),
        });
    }

    if report.failed_operations > 0 {
        return Err(RestoreRunnerError::Failed {
            backup_id: report.backup_id.clone(),
            failed_operations: report.failed_operations,
        });
    }

    if report.ready {
        return Ok(());
    }

    Err(RestoreRunnerError::NotReady {
        backup_id: journal.backup_id.clone(),
        reasons: report.blocked_reasons.clone(),
    })
}

// Convert an unavailable native runner command into the shared fail-closed error.
fn enforce_restore_run_command_available(
    preview: &RestoreApplyCommandPreview,
) -> Result<(), RestoreRunnerError> {
    if preview.command_available {
        return Ok(());
    }

    Err(restore_command_unavailable_error(preview))
}

// Build a shared command-unavailable error from a preview.
fn restore_command_unavailable_error(preview: &RestoreApplyCommandPreview) -> RestoreRunnerError {
    RestoreRunnerError::CommandUnavailable {
        backup_id: preview.backup_id.clone(),
        operation_available: preview.operation_available,
        complete: preview.complete,
        blocked_reasons: preview.blocked_reasons.clone(),
    }
}

// Render process exit status without relying on platform-specific internals.
fn exit_status_label(status: std::process::ExitStatus) -> String {
    status
        .code()
        .map_or_else(|| "signal".to_string(), |code| code.to_string())
}

// Extract the uploaded target snapshot ID from dfx upload output.
pub fn parse_uploaded_snapshot_id(output: &str) -> Option<String> {
    output
        .lines()
        .filter_map(|line| line.split_once(':').map(|(_, value)| value.trim()))
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

// Ensure a runner claim still matches the operation it previewed.
fn enforce_apply_claim_sequence(
    expected: usize,
    journal: &RestoreApplyJournal,
) -> Result<(), RestoreRunnerError> {
    let actual = journal
        .next_transition_operation()
        .map(|operation| operation.sequence);

    if actual == Some(expected) {
        return Ok(());
    }

    Err(RestoreRunnerError::ClaimSequenceMismatch { expected, actual })
}

// Read and validate a restore apply journal from disk.
fn read_apply_journal_file(path: &Path) -> Result<RestoreApplyJournal, RestoreRunnerError> {
    let data = fs::read_to_string(path)?;
    let journal: RestoreApplyJournal = serde_json::from_str(&data)?;
    journal.validate()?;
    Ok(journal)
}

// Return the caller-supplied journal update marker or the current placeholder.
fn state_updated_at(updated_at: Option<&String>) -> String {
    updated_at.cloned().unwrap_or_else(timestamp_placeholder)
}

// Return a placeholder timestamp until the runner owns a clock abstraction.
fn timestamp_placeholder() -> String {
    "unknown".to_string()
}

// Persist the restore apply journal to its canonical runner path.
fn write_apply_journal_file(
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

struct RestoreJournalLock {
    path: PathBuf,
}

impl RestoreJournalLock {
    // Acquire an atomic sidecar lock for mutating restore runner operations.
    fn acquire(journal_path: &Path) -> Result<Self, RestoreRunnerError> {
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
