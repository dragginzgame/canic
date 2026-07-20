mod manifest;
mod operations;
mod types;

pub use types::*;

use crate::{
    execution::{
        BackupExecutionJournal, BackupExecutionOperationReceipt,
        BackupExecutionOperationReceiptOutcome, BackupExecutionOperationState,
    },
    persistence::{BackupLayout, CommandLifetimeLock, CommandLifetimeLockError, JournalLock},
    plan::{BackupOperationKind, BackupPlan},
    timestamp::{current_timestamp_marker, state_updated_at, timestamp_marker, timestamp_seconds},
};
use operations::{
    execute_operation_receipt, operation_target, persist_created_snapshot,
    reconcile_pending_artifact_verification, reconcile_pending_download, recorded_snapshot_receipt,
};
use std::collections::{BTreeMap, BTreeSet};

const PREFLIGHT_TTL_SECONDS: u64 = 300;

#[cfg(test)]
pub(crate) fn build_manifest_for_test(
    config: &BackupRunnerConfig,
    plan: &BackupPlan,
    journal: &crate::journal::DownloadJournal,
) -> Result<crate::manifest::DeploymentBackupManifest, BackupRunnerError> {
    manifest::build_manifest(config, plan, journal)
}

/// Execute a persisted backup plan through an injected host executor.
pub fn backup_run_execute_with_executor(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
) -> Result<BackupRunResponse, BackupRunnerError> {
    backup_run_execute_with_terminal_writer(config, executor, &mut |layout, journal| {
        layout.write_execution_journal(journal)
    })
}

#[cfg(all(test, unix))]
pub(crate) fn backup_run_execute_with_terminal_barriers(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    mut barriers: impl FnMut(crate::persistence::DurableWriteBarrier),
) -> Result<BackupRunResponse, BackupRunnerError> {
    backup_run_execute_with_terminal_writer(config, executor, &mut |layout, journal| {
        layout.write_execution_journal_at_barriers(journal, &mut barriers)
    })
}

fn backup_run_execute_with_terminal_writer(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    terminal_writer: &mut impl FnMut(
        &BackupLayout,
        &BackupExecutionJournal,
    ) -> Result<(), crate::persistence::PersistenceError>,
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
    reject_premature_manifest(&layout, &journal)?;

    accept_preflight_if_needed(config, executor, &layout, &mut plan, &mut journal)?;
    execute_ready_operations(
        config,
        executor,
        &layout,
        &plan,
        &mut journal,
        terminal_writer,
    )
}

fn reject_premature_manifest(
    layout: &BackupLayout,
    journal: &BackupExecutionJournal,
) -> Result<(), BackupRunnerError> {
    if !layout.manifest_path().exists() {
        return Ok(());
    }
    let finalize = journal
        .operations
        .iter()
        .find(|operation| operation.kind == BackupOperationKind::FinalizeManifest)
        .ok_or(BackupRunnerError::NoReadyOperation)?;
    if matches!(
        finalize.state,
        BackupExecutionOperationState::Pending
            | BackupExecutionOperationState::Failed
            | BackupExecutionOperationState::Completed
    ) {
        return Ok(());
    }
    Err(BackupRunnerError::PrematureManifest {
        sequence: finalize.sequence,
        state: finalize.state.clone(),
    })
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
    terminal_writer: &mut impl FnMut(
        &BackupLayout,
        &BackupExecutionJournal,
    ) -> Result<(), crate::persistence::PersistenceError>,
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
        let preparation = prepare_backup_operation(
            config,
            executor,
            layout,
            plan,
            journal,
            &operation,
            &mut command_lock,
        );
        let reconciled_receipt = match preparation {
            Ok(receipt) => receipt,
            Err(error) => {
                let error = finish_preparation_command_lock(&operation, &mut command_lock, error)?;
                return Err(failure_after_containment(
                    config,
                    executor,
                    layout,
                    plan,
                    journal,
                    &operation,
                    terminal_writer,
                    error,
                ));
            }
        };
        if let Some(receipt) = reconciled_receipt {
            finish_backup_command_lock(&operation, command_lock.take())?;
            journal.record_operation_receipt(receipt)?;
            terminal_writer(layout, journal)?;
            executed.push(BackupRunExecutedOperation::completed(&operation));
            continue;
        }

        let operation_result = execute_operation_receipt(
            config,
            executor,
            layout,
            plan,
            journal,
            &operation,
            command_lock.as_ref().map(CommandLifetimeLock::handle),
        );
        finish_backup_command_lock(&operation, command_lock)?;

        match operation_result {
            Ok(receipt) => {
                journal.record_operation_receipt(receipt)?;
                terminal_writer(layout, journal)?;
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
                terminal_writer(layout, journal)?;
                executed.push(BackupRunExecutedOperation::failed(&operation));
                return Err(failure_after_containment(
                    config,
                    executor,
                    layout,
                    plan,
                    journal,
                    &operation,
                    terminal_writer,
                    error,
                ));
            }
        }
    }
}

fn finish_preparation_command_lock(
    operation: &crate::execution::BackupExecutionJournalOperation,
    command_lock: &mut Option<CommandLifetimeLock>,
    primary: BackupRunnerError,
) -> Result<BackupRunnerError, BackupRunnerError> {
    let Some(command_lock) = command_lock.take() else {
        return Ok(primary);
    };
    match command_lock
        .finish()
        .map_err(|error| backup_command_lock_error(operation, error))
    {
        Ok(()) => Ok(primary),
        Err(containment) => Err(BackupRunnerError::FailureContainmentFailed {
            primary: Box::new(primary),
            containment: Box::new(containment),
        }),
    }
}

fn prepare_backup_operation(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &mut BackupExecutionJournal,
    operation: &crate::execution::BackupExecutionJournalOperation,
    command_lock: &mut Option<CommandLifetimeLock>,
) -> Result<Option<BackupExecutionOperationReceipt>, BackupRunnerError> {
    if operation.state == BackupExecutionOperationState::Pending {
        if let Some(completed_status) = lifecycle_completed_status(&operation.kind) {
            reconcile_pending_lifecycle(executor, journal, operation, completed_status)
        } else if operation.kind == BackupOperationKind::CreateSnapshot {
            reconcile_pending_snapshot_create(executor, layout, plan, journal, operation)
        } else if operation.kind == BackupOperationKind::DownloadSnapshot {
            reconcile_pending_download(layout, journal, operation)
        } else if operation.kind == BackupOperationKind::VerifyArtifact {
            reconcile_pending_artifact_verification(layout, journal, operation)
        } else {
            reject_unknown_backup_command_outcome(operation, command_lock.take())?;
            Ok(None)
        }
    } else if let Some(completed_status) = lifecycle_completed_status(&operation.kind) {
        prepare_lifecycle_attempt(
            config,
            executor,
            layout,
            journal,
            operation,
            completed_status,
        )
    } else if operation.kind == BackupOperationKind::CreateSnapshot {
        prepare_snapshot_create_attempt(config, executor, layout, plan, journal, operation)
    } else {
        journal.mark_operation_pending_at(
            operation.sequence,
            Some(state_updated_at(config.updated_at.as_ref())),
        )?;
        layout.write_execution_journal(journal)?;
        Ok(None)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "failure containment needs the existing runner authorities and the primary typed cause"
)]
fn failure_after_containment(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &mut BackupExecutionJournal,
    primary_operation: &crate::execution::BackupExecutionJournalOperation,
    terminal_writer: &mut impl FnMut(
        &BackupLayout,
        &BackupExecutionJournal,
    ) -> Result<(), crate::persistence::PersistenceError>,
    primary: BackupRunnerError,
) -> BackupRunnerError {
    if !journal.restart_required && primary_operation.kind != BackupOperationKind::Stop {
        return primary;
    }
    if primary_operation.kind == BackupOperationKind::Stop
        && matches!(
            &primary,
            BackupRunnerError::CanisterStatusFailed { .. }
                | BackupRunnerError::CanisterStatusUnsettled { .. }
        )
    {
        return primary;
    }
    match contain_backup_failure(
        config,
        executor,
        layout,
        plan,
        journal,
        primary_operation,
        terminal_writer,
    ) {
        Ok(()) => primary,
        Err(containment) => BackupRunnerError::FailureContainmentFailed {
            primary: Box::new(primary),
            containment: Box::new(containment),
        },
    }
}

fn contain_backup_failure(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &mut BackupExecutionJournal,
    primary_operation: &crate::execution::BackupExecutionJournalOperation,
    terminal_writer: &mut impl FnMut(
        &BackupLayout,
        &BackupExecutionJournal,
    ) -> Result<(), crate::persistence::PersistenceError>,
) -> Result<(), BackupRunnerError> {
    reconcile_failed_stop_before_containment(
        config,
        executor,
        layout,
        journal,
        primary_operation,
        terminal_writer,
    )?;
    let rearm_snapshot_phase = matches!(
        primary_operation.kind,
        BackupOperationKind::Stop | BackupOperationKind::CreateSnapshot
    );

    while let Some(operation) = journal.next_failure_containment_start().cloned() {
        let mut command_lock = backup_command_lock(layout, &operation)?;
        let reconcile_previous_attempt = operation.state == BackupExecutionOperationState::Pending
            || operation.state == BackupExecutionOperationState::Failed
            || journal.operation_receipts.iter().any(|receipt| {
                receipt.sequence == operation.sequence
                    && receipt.outcome == BackupExecutionOperationReceiptOutcome::Failed
            });
        if operation.state != BackupExecutionOperationState::Pending {
            journal.mark_failure_containment_start_pending_at(
                operation.sequence,
                Some(state_updated_at(config.updated_at.as_ref())),
            )?;
            layout.write_execution_journal(journal)?;
        }
        let pending_operation = journal
            .operations
            .iter()
            .find(|candidate| candidate.sequence == operation.sequence)
            .cloned()
            .ok_or(
                crate::execution::BackupExecutionJournalError::OperationNotFound(
                    operation.sequence,
                ),
            )?;
        let reconciled_receipt = if reconcile_previous_attempt {
            reconcile_pending_lifecycle(
                executor,
                journal,
                &pending_operation,
                BackupRunnerCanisterStatus::Running,
            )?
        } else {
            None
        };
        let result = if let Some(receipt) = reconciled_receipt {
            finish_backup_command_lock(&pending_operation, command_lock.take())?;
            Ok(receipt)
        } else {
            let result = execute_operation_receipt(
                config,
                executor,
                layout,
                plan,
                journal,
                &pending_operation,
                command_lock.as_ref().map(CommandLifetimeLock::handle),
            );
            if let Some(command_lock) = command_lock {
                command_lock
                    .finish()
                    .map_err(|error| backup_command_lock_error(&pending_operation, error))?;
            }
            result
        };

        match result {
            Ok(receipt) => {
                journal.record_operation_receipt(receipt)?;
                if rearm_snapshot_phase {
                    journal.rearm_after_failure_containment(
                        operation.sequence,
                        Some(state_updated_at(config.updated_at.as_ref())),
                    )?;
                }
                terminal_writer(layout, journal)?;
            }
            Err(error) => {
                let receipt = BackupExecutionOperationReceipt::failed(
                    journal,
                    &pending_operation,
                    Some(state_updated_at(config.updated_at.as_ref())),
                    error.to_string(),
                );
                journal.record_operation_receipt(receipt)?;
                terminal_writer(layout, journal)?;
                return Err(error);
            }
        }
    }

    Ok(())
}

fn reconcile_failed_stop_before_containment(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    journal: &mut BackupExecutionJournal,
    primary_operation: &crate::execution::BackupExecutionJournalOperation,
    terminal_writer: &mut impl FnMut(
        &BackupLayout,
        &BackupExecutionJournal,
    ) -> Result<(), crate::persistence::PersistenceError>,
) -> Result<(), BackupRunnerError> {
    if primary_operation.kind != BackupOperationKind::Stop {
        return Ok(());
    }
    let target = operation_target(primary_operation)?;
    let status = executor.canister_status(&target).map_err(|error| {
        BackupRunnerError::CanisterStatusFailed {
            sequence: primary_operation.sequence,
            status: error.status,
            message: error.message,
        }
    })?;
    match status {
        BackupRunnerCanisterStatus::Running => Ok(()),
        BackupRunnerCanisterStatus::Stopping => Err(BackupRunnerError::CanisterStatusUnsettled {
            sequence: primary_operation.sequence,
            operation_id: primary_operation.operation_id.clone(),
            status: status.label(),
        }),
        BackupRunnerCanisterStatus::Stopped => {
            journal.mark_operation_pending_at(
                primary_operation.sequence,
                Some(state_updated_at(config.updated_at.as_ref())),
            )?;
            layout.write_execution_journal(journal)?;
            let pending_operation = journal
                .operations
                .iter()
                .find(|operation| operation.sequence == primary_operation.sequence)
                .cloned()
                .ok_or(
                    crate::execution::BackupExecutionJournalError::OperationNotFound(
                        primary_operation.sequence,
                    ),
                )?;
            journal.record_operation_receipt(BackupExecutionOperationReceipt::completed(
                journal,
                &pending_operation,
                Some(current_timestamp_marker()),
            ))?;
            terminal_writer(layout, journal)?;
            Ok(())
        }
    }
}

fn prepare_lifecycle_attempt(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    journal: &mut BackupExecutionJournal,
    operation: &crate::execution::BackupExecutionJournalOperation,
    completed_status: BackupRunnerCanisterStatus,
) -> Result<Option<BackupExecutionOperationReceipt>, BackupRunnerError> {
    let reconcile_previous_attempt = operation.state == BackupExecutionOperationState::Failed
        || journal.operation_receipts.iter().any(|receipt| {
            receipt.sequence == operation.sequence
                && receipt.outcome == BackupExecutionOperationReceiptOutcome::Failed
        });
    journal.mark_operation_pending_at(
        operation.sequence,
        Some(state_updated_at(config.updated_at.as_ref())),
    )?;
    layout.write_execution_journal(journal)?;
    if !reconcile_previous_attempt {
        return Ok(None);
    }
    let pending_operation = journal
        .operations
        .iter()
        .find(|candidate| candidate.sequence == operation.sequence)
        .cloned()
        .ok_or(
            crate::execution::BackupExecutionJournalError::OperationNotFound(operation.sequence),
        )?;
    reconcile_pending_lifecycle(executor, journal, &pending_operation, completed_status)
}

fn prepare_snapshot_create_attempt(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &mut BackupExecutionJournal,
    operation: &crate::execution::BackupExecutionJournalOperation,
) -> Result<Option<BackupExecutionOperationReceipt>, BackupRunnerError> {
    let recovering_previous_attempt = operation.snapshot_ids_before.is_some();
    let snapshot_ids_before = if recovering_previous_attempt {
        operation.snapshot_ids_before.clone().ok_or(
            crate::execution::BackupExecutionJournalError::MissingField(
                "operations[].snapshot_ids_before",
            ),
        )?
    } else {
        let target = operation_target(operation)?;
        let snapshots = observe_snapshot_inventory(executor, operation, &target)?;
        let mut snapshot_ids = snapshots
            .into_iter()
            .map(|snapshot| snapshot.snapshot_id)
            .collect::<Vec<_>>();
        snapshot_ids.sort();
        snapshot_ids
    };
    journal.mark_snapshot_create_pending_at(
        operation.sequence,
        Some(state_updated_at(config.updated_at.as_ref())),
        snapshot_ids_before,
    )?;
    layout.write_execution_journal(journal)?;
    if !recovering_previous_attempt {
        return Ok(None);
    }
    let pending_operation = journal
        .operations
        .iter()
        .find(|candidate| candidate.sequence == operation.sequence)
        .cloned()
        .ok_or(
            crate::execution::BackupExecutionJournalError::OperationNotFound(operation.sequence),
        )?;
    reconcile_pending_snapshot_create(executor, layout, plan, journal, &pending_operation)
}

fn reconcile_pending_snapshot_create(
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &crate::execution::BackupExecutionJournalOperation,
) -> Result<Option<BackupExecutionOperationReceipt>, BackupRunnerError> {
    let target = operation_target(operation)?;
    if let Some(receipt) = recorded_snapshot_receipt(layout, plan, journal, operation, &target)? {
        return Ok(Some(receipt));
    }
    let baseline = operation.snapshot_ids_before.as_ref().ok_or(
        crate::execution::BackupExecutionJournalError::MissingField(
            "operations[].snapshot_ids_before",
        ),
    )?;
    let baseline = baseline.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let observed = observe_snapshot_inventory(executor, operation, &target)?
        .into_iter()
        .map(|snapshot| (snapshot.snapshot_id.clone(), snapshot))
        .collect::<BTreeMap<_, _>>();
    let missing = baseline
        .iter()
        .filter(|snapshot_id| !observed.contains_key(**snapshot_id))
        .map(|snapshot_id| (*snapshot_id).to_string())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(BackupRunnerError::SnapshotInventoryLostBaseline {
            sequence: operation.sequence,
            operation_id: operation.operation_id.clone(),
            snapshot_ids: missing,
        });
    }
    let mut created = observed
        .into_iter()
        .filter(|(snapshot_id, _)| !baseline.contains(snapshot_id.as_str()))
        .collect::<Vec<_>>();
    match created.len() {
        0 => Ok(None),
        1 => {
            let snapshot = created.pop().expect("one snapshot candidate").1;
            persist_created_snapshot(layout, plan, journal, operation, &target, snapshot).map(Some)
        }
        _ => Err(BackupRunnerError::SnapshotIdentityAmbiguous {
            sequence: operation.sequence,
            operation_id: operation.operation_id.clone(),
            snapshot_ids: created
                .into_iter()
                .map(|(snapshot_id, _)| snapshot_id)
                .collect(),
        }),
    }
}

fn observe_snapshot_inventory(
    executor: &mut impl BackupRunnerExecutor,
    operation: &crate::execution::BackupExecutionJournalOperation,
    target: &str,
) -> Result<Vec<BackupRunnerSnapshot>, BackupRunnerError> {
    let snapshots = executor.snapshot_inventory(target).map_err(|error| {
        BackupRunnerError::SnapshotInventoryFailed {
            sequence: operation.sequence,
            status: error.status,
            message: error.message,
        }
    })?;
    let mut identities = BTreeSet::new();
    for snapshot in &snapshots {
        if snapshot.snapshot_id.trim().is_empty() {
            return Err(BackupRunnerError::InvalidSnapshotIdentity {
                sequence: operation.sequence,
                operation_id: operation.operation_id.clone(),
            });
        }
        if !identities.insert(snapshot.snapshot_id.as_str()) {
            return Err(BackupRunnerError::DuplicateSnapshotIdentity {
                sequence: operation.sequence,
                operation_id: operation.operation_id.clone(),
                snapshot_id: snapshot.snapshot_id.clone(),
            });
        }
    }
    Ok(snapshots)
}

fn reconcile_pending_lifecycle(
    executor: &mut impl BackupRunnerExecutor,
    journal: &BackupExecutionJournal,
    operation: &crate::execution::BackupExecutionJournalOperation,
    completed_status: BackupRunnerCanisterStatus,
) -> Result<Option<BackupExecutionOperationReceipt>, BackupRunnerError> {
    let target = operation.target_canister_id.as_deref().ok_or(
        BackupRunnerError::MissingOperationTarget {
            sequence: operation.sequence,
        },
    )?;
    let status = executor.canister_status(target).map_err(|error| {
        BackupRunnerError::CanisterStatusFailed {
            sequence: operation.sequence,
            status: error.status,
            message: error.message,
        }
    })?;
    if status == completed_status {
        return Ok(Some(BackupExecutionOperationReceipt::completed(
            journal,
            operation,
            Some(current_timestamp_marker()),
        )));
    }
    if status == BackupRunnerCanisterStatus::Stopping {
        return Err(BackupRunnerError::CanisterStatusUnsettled {
            sequence: operation.sequence,
            operation_id: operation.operation_id.clone(),
            status: status.label(),
        });
    }
    Ok(None)
}

const fn lifecycle_completed_status(
    kind: &BackupOperationKind,
) -> Option<BackupRunnerCanisterStatus> {
    match kind {
        BackupOperationKind::Stop => Some(BackupRunnerCanisterStatus::Stopped),
        BackupOperationKind::Start => Some(BackupRunnerCanisterStatus::Running),
        _ => None,
    }
}

fn finish_backup_command_lock(
    operation: &crate::execution::BackupExecutionJournalOperation,
    command_lock: Option<CommandLifetimeLock>,
) -> Result<(), BackupRunnerError> {
    let Some(command_lock) = command_lock else {
        if backup_operation_uses_command_lock(&operation.kind) {
            return Err(BackupRunnerError::MissingCommandLifetime {
                sequence: operation.sequence,
                operation_id: operation.operation_id.clone(),
            });
        }
        return Ok(());
    };
    command_lock
        .finish()
        .map_err(|error| backup_command_lock_error(operation, error))
}

const fn backup_operation_uses_command_lock(kind: &BackupOperationKind) -> bool {
    matches!(
        kind,
        BackupOperationKind::Stop
            | BackupOperationKind::CreateSnapshot
            | BackupOperationKind::Start
            | BackupOperationKind::DownloadSnapshot
    )
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
    if !backup_operation_uses_command_lock(&operation.kind) {
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
