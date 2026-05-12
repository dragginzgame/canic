mod manifest;
mod operations;
mod support;
mod types;

pub use types::*;

use crate::{
    execution::{BackupExecutionJournal, BackupExecutionOperationState},
    persistence::BackupLayout,
    plan::BackupPlan,
};
use operations::execute_operation_receipt;
use support::{BackupRunLock, state_updated_at, timestamp_marker, timestamp_seconds};

const PREFLIGHT_TTL_SECONDS: u64 = 300;

/// Execute a persisted backup plan through an injected host executor.
pub fn backup_run_execute_with_executor(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
) -> Result<BackupRunResponse, BackupRunnerError> {
    let layout = BackupLayout::new(config.out.clone());
    let _lock = BackupRunLock::acquire(&layout.execution_journal_path())?;
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

        if operation.state != BackupExecutionOperationState::Pending {
            journal.mark_operation_pending_at(
                operation.sequence,
                Some(state_updated_at(config.updated_at.as_ref())),
            )?;
            layout.write_execution_journal(journal)?;
        }

        match execute_operation_receipt(config, executor, layout, plan, journal, &operation) {
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
