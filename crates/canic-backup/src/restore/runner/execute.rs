use super::{
    RestoreApplyCommandOutputPair, RestoreApplyJournal, RestoreApplyOperationKind,
    RestoreApplyOperationReceipt,
    constants::{
        RESTORE_RUN_COMMAND_EXIT_PREFIX, RESTORE_RUN_MISSING_UPLOADED_SNAPSHOT_ID,
        RESTORE_RUN_OUTPUT_RECEIPT_LIMIT, RESTORE_RUN_STOPPED_PRECONDITION_FAILED,
    },
    io::{RestoreJournalLock, read_apply_journal_file, state_updated_at, write_apply_journal_file},
    precondition::enforce_stopped_canister_precondition,
    status::{
        enforce_restore_run_command_available, enforce_restore_run_executable,
        parse_uploaded_snapshot_id, restore_command_unavailable_error,
        restore_run_max_steps_reached, restore_run_next_action, restore_run_stopped_reason,
    },
    types::{
        RestoreRunExecutedOperation, RestoreRunOperationReceipt, RestoreRunPreparedOperation,
        RestoreRunResponse, RestoreRunResponseMode, RestoreRunStepOutcome,
        RestoreRunnerCommandExecutor, RestoreRunnerConfig, RestoreRunnerError,
        RestoreRunnerOutcome, RestoreStoppedPreconditionFailure,
    },
};

/// Execute ready restore apply journal operations through an injected command executor.
pub fn restore_run_execute_with_executor(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
) -> Result<RestoreRunResponse, RestoreRunnerError> {
    let run = restore_run_execute_result_with_executor(config, executor)?;
    if let Some(error) = run.error {
        return Err(error);
    }

    Ok(run.response)
}

// Execute ready restore apply operations and retain any deferred runner error.
pub fn restore_run_execute_result_with_executor(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
) -> Result<RestoreRunnerOutcome, RestoreRunnerError> {
    let _lock = RestoreJournalLock::acquire(&config.journal)?;
    let mut journal = read_apply_journal_file(&config.journal)?;
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
            )));
        }

        enforce_restore_run_executable(&journal, &report)?;
        let prepared = restore_run_prepare_next_operation(config, &mut journal)?;
        let sequence = prepared.sequence;
        match restore_run_execute_prepared_operation(config, executor, &mut journal, prepared)? {
            RestoreRunStepOutcome::Completed {
                executed_operation,
                operation_receipt,
            } => {
                executed_operations.push(executed_operation);
                operation_receipts.push(operation_receipt);
            }
            RestoreRunStepOutcome::Failed {
                executed_operation,
                operation_receipt,
                status,
            } => {
                executed_operations.push(executed_operation);
                operation_receipts.push(operation_receipt);
                let response = restore_run_execute_summary(
                    &journal,
                    executed_operations,
                    operation_receipts,
                    false,
                    config.updated_at.as_ref(),
                );
                return Ok(RestoreRunnerOutcome {
                    response,
                    error: Some(RestoreRunnerError::CommandFailed { sequence, status }),
                });
            }
        }
    }
}

// Claim the next renderable operation and persist the pending state.
fn restore_run_prepare_next_operation(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
) -> Result<RestoreRunPreparedOperation, RestoreRunnerError> {
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

    enforce_apply_claim_sequence(sequence, journal)?;
    journal
        .mark_operation_pending_at(sequence, Some(state_updated_at(config.updated_at.as_ref())))?;
    write_apply_journal_file(&config.journal, journal)?;

    Ok(RestoreRunPreparedOperation {
        operation,
        command,
        sequence,
        attempt,
    })
}

// Execute one claimed operation and commit either success or failure.
fn restore_run_execute_prepared_operation(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    journal: &mut RestoreApplyJournal,
    prepared: RestoreRunPreparedOperation,
) -> Result<RestoreRunStepOutcome, RestoreRunnerError> {
    if prepared.command.requires_stopped_canister
        && let Some(outcome) = enforce_stopped_canister_precondition(
            config,
            executor,
            &prepared.operation,
            prepared.attempt,
            config.updated_at.as_ref(),
        )?
    {
        return restore_run_commit_precondition_failure(config, journal, prepared, outcome);
    }

    let output = executor.execute(&prepared.command)?;
    let status_label = output.status;
    let output_pair = RestoreApplyCommandOutputPair::from_bytes(
        &output.stdout,
        &output.stderr,
        RESTORE_RUN_OUTPUT_RECEIPT_LIMIT,
    );

    if output.success {
        let is_upload_snapshot =
            prepared.operation.operation == RestoreApplyOperationKind::UploadSnapshot;
        let uploaded_snapshot_id = is_upload_snapshot
            .then(|| parse_uploaded_snapshot_id(&String::from_utf8_lossy(&output.stdout)))
            .flatten();
        if is_upload_snapshot && uploaded_snapshot_id.is_none() {
            return restore_run_commit_missing_uploaded_snapshot_id(
                config,
                journal,
                prepared,
                output_pair,
            );
        }

        return restore_run_commit_command_success(
            config,
            journal,
            prepared,
            status_label,
            output_pair,
            uploaded_snapshot_id,
        );
    }

    restore_run_commit_command_failure(config, journal, prepared, status_label, output_pair)
}

// Commit a successful upload command whose output is missing the required snapshot id.
fn restore_run_commit_missing_uploaded_snapshot_id(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
    prepared: RestoreRunPreparedOperation,
    output_pair: RestoreApplyCommandOutputPair,
) -> Result<RestoreRunStepOutcome, RestoreRunnerError> {
    let failed_updated_at = state_updated_at(config.updated_at.as_ref());
    let status = RESTORE_RUN_MISSING_UPLOADED_SNAPSHOT_ID.to_string();
    journal.mark_operation_failed_at(
        prepared.sequence,
        status.clone(),
        Some(failed_updated_at.clone()),
    )?;
    journal.record_operation_receipt(RestoreApplyOperationReceipt::command_failed(
        &prepared.operation,
        prepared.command.clone(),
        status.clone(),
        Some(failed_updated_at.clone()),
        output_pair,
        prepared.attempt,
        status.clone(),
    ))?;
    write_apply_journal_file(&config.journal, journal)?;

    Ok(RestoreRunStepOutcome::Failed {
        executed_operation: RestoreRunExecutedOperation::failed(
            prepared.operation.clone(),
            prepared.command.clone(),
            RESTORE_RUN_MISSING_UPLOADED_SNAPSHOT_ID.to_string(),
        ),
        operation_receipt: RestoreRunOperationReceipt::failed(
            prepared.operation,
            prepared.command,
            status.clone(),
            Some(failed_updated_at),
        ),
        status,
    })
}

// Commit a stopped-canister precondition failure for a claimed operation.
fn restore_run_commit_precondition_failure(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
    prepared: RestoreRunPreparedOperation,
    outcome: RestoreStoppedPreconditionFailure,
) -> Result<RestoreRunStepOutcome, RestoreRunnerError> {
    let failed_updated_at = state_updated_at(config.updated_at.as_ref());
    journal.mark_operation_failed_at(
        prepared.sequence,
        outcome.failure_reason.clone(),
        Some(failed_updated_at.clone()),
    )?;
    journal.record_operation_receipt(RestoreApplyOperationReceipt::command_failed(
        &prepared.operation,
        outcome.command.clone(),
        outcome.status_label.clone(),
        Some(failed_updated_at.clone()),
        outcome.output,
        prepared.attempt,
        outcome.failure_reason,
    ))?;
    write_apply_journal_file(&config.journal, journal)?;

    Ok(RestoreRunStepOutcome::Failed {
        executed_operation: RestoreRunExecutedOperation::failed(
            prepared.operation.clone(),
            outcome.command.clone(),
            outcome.status_label.clone(),
        ),
        operation_receipt: RestoreRunOperationReceipt::failed(
            prepared.operation,
            outcome.command,
            outcome.status_label,
            Some(failed_updated_at),
        ),
        status: RESTORE_RUN_STOPPED_PRECONDITION_FAILED.to_string(),
    })
}

// Commit a successful process command for a claimed operation.
fn restore_run_commit_command_success(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
    prepared: RestoreRunPreparedOperation,
    status_label: String,
    output_pair: RestoreApplyCommandOutputPair,
    uploaded_snapshot_id: Option<String>,
) -> Result<RestoreRunStepOutcome, RestoreRunnerError> {
    let completed_updated_at = state_updated_at(config.updated_at.as_ref());
    journal.mark_operation_completed_at(prepared.sequence, Some(completed_updated_at.clone()))?;
    journal.record_operation_receipt(RestoreApplyOperationReceipt::command_completed(
        &prepared.operation,
        prepared.command.clone(),
        status_label.clone(),
        Some(completed_updated_at.clone()),
        output_pair,
        prepared.attempt,
        uploaded_snapshot_id,
    ))?;
    write_apply_journal_file(&config.journal, journal)?;

    Ok(RestoreRunStepOutcome::Completed {
        executed_operation: RestoreRunExecutedOperation::completed(
            prepared.operation.clone(),
            prepared.command.clone(),
            status_label.clone(),
        ),
        operation_receipt: RestoreRunOperationReceipt::completed(
            prepared.operation,
            prepared.command,
            status_label,
            Some(completed_updated_at),
        ),
    })
}

// Commit a failed process command for a claimed operation.
fn restore_run_commit_command_failure(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
    prepared: RestoreRunPreparedOperation,
    status_label: String,
    output_pair: RestoreApplyCommandOutputPair,
) -> Result<RestoreRunStepOutcome, RestoreRunnerError> {
    let failed_updated_at = state_updated_at(config.updated_at.as_ref());
    let failure_reason = format!("{RESTORE_RUN_COMMAND_EXIT_PREFIX}-{status_label}");
    journal.mark_operation_failed_at(
        prepared.sequence,
        failure_reason.clone(),
        Some(failed_updated_at.clone()),
    )?;
    journal.record_operation_receipt(RestoreApplyOperationReceipt::command_failed(
        &prepared.operation,
        prepared.command.clone(),
        status_label.clone(),
        Some(failed_updated_at.clone()),
        output_pair,
        prepared.attempt,
        failure_reason,
    ))?;
    write_apply_journal_file(&config.journal, journal)?;

    Ok(RestoreRunStepOutcome::Failed {
        executed_operation: RestoreRunExecutedOperation::failed(
            prepared.operation.clone(),
            prepared.command.clone(),
            status_label.clone(),
        ),
        operation_receipt: RestoreRunOperationReceipt::failed(
            prepared.operation,
            prepared.command,
            status_label.clone(),
            Some(failed_updated_at),
        ),
        status: status_label,
    })
}

// Build the final native runner execution summary.
fn restore_run_execute_summary(
    journal: &RestoreApplyJournal,
    executed_operations: Vec<RestoreRunExecutedOperation>,
    operation_receipts: Vec<RestoreRunOperationReceipt>,
    max_steps_reached: bool,
    requested_state_updated_at: Option<&String>,
) -> RestoreRunResponse {
    let report = journal.report();
    let executed_operation_count = executed_operations.len();
    let stopped_reason = restore_run_stopped_reason(&report, max_steps_reached, true);
    let next_action = restore_run_next_action(&report);

    let mut response = RestoreRunResponse::from_report(
        journal.backup_id.clone(),
        report,
        RestoreRunResponseMode::execute(stopped_reason, next_action),
    );
    response.set_requested_state_updated_at(requested_state_updated_at);
    response.max_steps_reached = Some(max_steps_reached);
    response.executed_operation_count = Some(executed_operation_count);
    response.executed_operations = executed_operations;
    response.set_operation_receipts(operation_receipts);
    response
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
