use super::{
    RestoreApplyCommandPreview, RestoreApplyJournal, RestoreApplyJournalReport,
    constants::*,
    types::{RestoreRunnerConfig, RestoreRunnerError},
};

// Check whether execute mode has reached its requested operation batch size.
pub(super) fn restore_run_max_steps_reached(
    config: &RestoreRunnerConfig,
    executed_operation_count: usize,
    report: &RestoreApplyJournalReport,
) -> bool {
    config.max_steps == Some(executed_operation_count) && !report.complete
}

// Classify why the native runner stopped for operator summaries.
pub(super) const fn restore_run_stopped_reason(
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
pub(super) const fn restore_run_next_action(
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
pub(super) fn enforce_restore_run_executable(
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
pub(super) fn enforce_restore_run_command_available(
    preview: &RestoreApplyCommandPreview,
) -> Result<(), RestoreRunnerError> {
    if preview.command_available {
        return Ok(());
    }

    Err(restore_command_unavailable_error(preview))
}

// Build a shared command-unavailable error from a preview.
pub(super) fn restore_command_unavailable_error(
    preview: &RestoreApplyCommandPreview,
) -> RestoreRunnerError {
    RestoreRunnerError::CommandUnavailable {
        backup_id: preview.backup_id.clone(),
        operation_available: preview.operation_available,
        complete: preview.complete,
        blocked_reasons: preview.blocked_reasons.clone(),
    }
}

// Extract the uploaded target snapshot ID from command output.
pub fn parse_uploaded_snapshot_id(output: &str) -> Option<String> {
    output
        .lines()
        .filter_map(|line| line.split_once(':').map(|(_, value)| value.trim()))
        .find(|value| !value.is_empty())
        .map(str::to_string)
}
