mod manifest;
mod operations;
mod types;

pub use types::*;

use crate::{
    execution::{BackupExecutionJournal, BackupExecutionOperationState},
    persistence::{BackupLayout, CommandLifetimeLock, CommandLifetimeLockError, JournalLock},
    plan::BackupPlan,
    timestamp::{state_updated_at, timestamp_marker, timestamp_seconds},
};
use operations::execute_operation_receipt;

const PREFLIGHT_TTL_SECONDS: u64 = 300;

/// Execute a persisted backup plan through an injected host executor.
pub fn backup_run_execute_with_executor(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
) -> Result<BackupRunResponse, BackupRunnerError> {
    let layout = BackupLayout::new(config.out.clone());
    let _lock = JournalLock::acquire(&layout.execution_journal_path())?;
    let mut plan = layout.read_backup_plan()?;
    let mut journal = if layout.execution_journal_path().is_file() {
        layout.read_execution_journal()?
    } else {
        let journal = BackupExecutionJournal::from_plan(&plan)?;
        layout.write_execution_journal(&journal)?;
        journal
    };
    layout.verify_execution_integrity()?;

    accept_preflight_if_needed(config, executor, &layout, &mut plan, &mut journal)?;
    execute_ready_operations(config, executor, &layout, &plan, &mut journal)
}

fn accept_preflight_if_needed(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &mut BackupPlan,
    journal: &mut BackupExecutionJournal,
) -> Result<(), BackupRunnerError> {
    if journal.preflight_accepted {
        return Ok(());
    }

    let validated_at = state_updated_at(config.updated_at.as_ref());
    let expires_at = timestamp_marker(timestamp_seconds(&validated_at) + PREFLIGHT_TTL_SECONDS);
    let preflight_id = format!("preflight-{}", plan.run_id);
    let receipts = executor
        .preflight_receipts(plan, &preflight_id, &validated_at, &expires_at)
        .map_err(|error| BackupRunnerError::PreflightFailed {
            status: error.status,
            message: error.message,
        })?;
    plan.apply_execution_preflight_receipts(&receipts, &validated_at)?;
    layout.write_backup_plan(plan)?;
    journal.accept_preflight_receipts_at(&receipts, Some(validated_at))?;
    layout.write_execution_journal(journal)?;
    Ok(())
}

fn execute_ready_operations(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &mut BackupExecutionJournal,
) -> Result<BackupRunResponse, BackupRunnerError> {
    let mut executed = Vec::new();

    loop {
        let summary = journal.resume_summary();
        if summary.completed_operations + summary.skipped_operations == summary.total_operations {
            return Ok(run_response(plan, journal, executed, false));
        }
        if config
            .max_steps
            .is_some_and(|max_steps| executed.len() >= max_steps)
        {
            return Ok(run_response(plan, journal, executed, true));
        }

        let operation = journal
            .next_ready_operation()
            .cloned()
            .ok_or(BackupRunnerError::NoReadyOperation)?;
        if operation.state == BackupExecutionOperationState::Blocked {
            return Err(BackupRunnerError::Blocked {
                reasons: operation.blocking_reasons,
            });
        }

        let mut command_lock = backup_command_lock(layout, &operation)?;
        if operation.state == BackupExecutionOperationState::Pending {
            reject_unknown_backup_command_outcome(&operation, command_lock.take())?;
        }

        journal.mark_operation_pending_at(
            operation.sequence,
            Some(state_updated_at(config.updated_at.as_ref())),
        )?;
        layout.write_execution_journal(journal)?;

        let operation_result = execute_operation_receipt(
            config,
            executor,
            layout,
            plan,
            journal,
            &operation,
            command_lock.as_ref().map(CommandLifetimeLock::handle),
        );
        if let Some(command_lock) = command_lock {
            command_lock
                .finish()
                .map_err(|error| backup_command_lock_error(&operation, error))?;
        }

        match operation_result {
            Ok(receipt) => {
                journal.record_operation_receipt(receipt)?;
                layout.write_execution_journal(journal)?;
                executed.push(BackupRunExecutedOperation::completed(&operation));
            }
            Err(error) => {
                let receipt = crate::execution::BackupExecutionOperationReceipt::failed(
                    journal,
                    &operation,
                    Some(state_updated_at(config.updated_at.as_ref())),
                    error.to_string(),
                );
                journal.record_operation_receipt(receipt)?;
                layout.write_execution_journal(journal)?;
                executed.push(BackupRunExecutedOperation::failed(&operation));
                return Err(error);
            }
        }
    }
}

fn reject_unknown_backup_command_outcome(
    operation: &crate::execution::BackupExecutionJournalOperation,
    command_lock: Option<CommandLifetimeLock>,
) -> Result<(), BackupRunnerError> {
    let Some(command_lock) = command_lock else {
        return Ok(());
    };
    let lock_path = command_lock.path().to_string_lossy().to_string();
    command_lock
        .finish()
        .map_err(|error| backup_command_lock_error(operation, error))?;
    Err(BackupRunnerError::CommandOutcomeUnknown {
        sequence: operation.sequence,
        operation_id: operation.operation_id.clone(),
        lock_path,
    })
}

fn backup_command_lock(
    layout: &BackupLayout,
    operation: &crate::execution::BackupExecutionJournalOperation,
) -> Result<Option<CommandLifetimeLock>, BackupRunnerError> {
    if !matches!(
        operation.kind,
        crate::plan::BackupOperationKind::Stop
            | crate::plan::BackupOperationKind::CreateSnapshot
            | crate::plan::BackupOperationKind::Start
            | crate::plan::BackupOperationKind::DownloadSnapshot
    ) {
        return Ok(None);
    }

    CommandLifetimeLock::acquire(&layout.execution_journal_path(), operation.sequence)
        .map(Some)
        .map_err(|error| backup_command_lock_error(operation, error))
}

fn backup_command_lock_error(
    operation: &crate::execution::BackupExecutionJournalOperation,
    error: CommandLifetimeLockError,
) -> BackupRunnerError {
    match error {
        CommandLifetimeLockError::InFlight { lock_path } => BackupRunnerError::CommandInFlight {
            sequence: operation.sequence,
            operation_id: operation.operation_id.clone(),
            lock_path,
        },
        CommandLifetimeLockError::UnsafeEntry { lock_path, kind } => {
            BackupRunnerError::CommandLockUnsafeEntry {
                sequence: operation.sequence,
                operation_id: operation.operation_id.clone(),
                lock_path,
                kind,
            }
        }
        CommandLifetimeLockError::Io(error) => BackupRunnerError::Io(error),
    }
}

fn run_response(
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    executed: Vec<BackupRunExecutedOperation>,
    max_steps_reached: bool,
) -> BackupRunResponse {
    let execution = journal.resume_summary();
    BackupRunResponse {
        run_id: plan.run_id.clone(),
        plan_id: plan.plan_id.clone(),
        backup_id: plan.run_id.clone(),
        complete: execution.completed_operations + execution.skipped_operations
            == execution.total_operations,
        max_steps_reached,
        executed_operation_count: executed.len(),
        executed_operations: executed,
        execution,
    }
}

#[cfg(test)]
mod tests;
