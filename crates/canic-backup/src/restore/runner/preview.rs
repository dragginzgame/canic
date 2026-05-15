use super::{
    RestoreApplyJournalError, RestoreApplyOperationState,
    io::{RestoreJournalLock, read_apply_journal_file, state_updated_at, write_apply_journal_file},
    status::{restore_run_next_action, restore_run_stopped_reason},
    types::{
        RestoreRunOperationReceipt, RestoreRunResponse, RestoreRunResponseMode,
        RestoreRunnerConfig, RestoreRunnerError,
    },
};

/// Build a no-mutation native restore runner preview from a journal file.
pub fn restore_run_dry_run(
    config: &RestoreRunnerConfig,
) -> Result<RestoreRunResponse, RestoreRunnerError> {
    let journal = read_apply_journal_file(&config.journal)?;
    let report = journal.report();
    let preview = journal.next_command_preview_with_config(&config.command);
    let stopped_reason = restore_run_stopped_reason(&report, false, false);
    let next_action = restore_run_next_action(&report);

    let mut response = RestoreRunResponse::from_report(
        journal.backup_id,
        report,
        RestoreRunResponseMode::dry_run(stopped_reason, next_action),
    );
    response.set_requested_state_updated_at(config.updated_at.as_ref());
    response.operation_available = Some(preview.operation_available);
    response.command_available = Some(preview.command_available);
    response.command = preview.command;
    Ok(response)
}

/// Recover a failed restore runner operation by moving it back to ready.
pub fn restore_run_retry_failed(
    config: &RestoreRunnerConfig,
) -> Result<RestoreRunResponse, RestoreRunnerError> {
    let _lock = RestoreJournalLock::acquire(&config.journal)?;
    let mut journal = read_apply_journal_file(&config.journal)?;
    let recovered_operation = journal
        .next_transition_operation()
        .filter(|operation| operation.state == RestoreApplyOperationState::Failed)
        .cloned()
        .ok_or(RestoreApplyJournalError::NoFailedOperation)?;

    let recovered_updated_at = state_updated_at(config.updated_at.as_ref());
    journal.retry_failed_operation_at(
        recovered_operation.sequence,
        Some(recovered_updated_at.clone()),
    )?;
    write_apply_journal_file(&config.journal, &journal)?;

    let report = journal.report();
    let next_action = restore_run_next_action(&report);
    let mut response = RestoreRunResponse::from_report(
        journal.backup_id,
        report,
        RestoreRunResponseMode::retry_failed(next_action),
    );
    response.set_requested_state_updated_at(config.updated_at.as_ref());
    response.set_operation_receipts(vec![RestoreRunOperationReceipt::recovered_failed(
        recovered_operation.clone(),
        Some(recovered_updated_at),
    )]);
    response.recovered_operation = Some(recovered_operation);
    Ok(response)
}

/// Recover an interrupted restore runner by unclaiming the pending operation.
pub fn restore_run_unclaim_pending(
    config: &RestoreRunnerConfig,
) -> Result<RestoreRunResponse, RestoreRunnerError> {
    let _lock = RestoreJournalLock::acquire(&config.journal)?;
    let mut journal = read_apply_journal_file(&config.journal)?;
    let recovered_operation = journal
        .next_transition_operation()
        .filter(|operation| operation.state == RestoreApplyOperationState::Pending)
        .cloned()
        .ok_or(RestoreApplyJournalError::NoPendingOperation)?;

    let recovered_updated_at = state_updated_at(config.updated_at.as_ref());
    journal.mark_next_operation_ready_at(Some(recovered_updated_at.clone()))?;
    write_apply_journal_file(&config.journal, &journal)?;

    let report = journal.report();
    let next_action = restore_run_next_action(&report);
    let mut response = RestoreRunResponse::from_report(
        journal.backup_id,
        report,
        RestoreRunResponseMode::unclaim_pending(next_action),
    );
    response.set_requested_state_updated_at(config.updated_at.as_ref());
    response.set_operation_receipts(vec![RestoreRunOperationReceipt::recovered_pending(
        recovered_operation.clone(),
        Some(recovered_updated_at),
    )]);
    response.recovered_operation = Some(recovered_operation);
    Ok(response)
}
