//! Module: runner::operations
//!
//! Responsibility: execute one validated backup runner operation.
//! Does not own: plan construction, journal validation, or command implementation.
//! Boundary: converts executor results into durable execution receipts.

mod artifacts;
mod journal;
mod path;

use crate::{
    execution::{
        BackupExecutionJournal, BackupExecutionJournalOperation, BackupExecutionOperationReceipt,
    },
    persistence::{BackupLayout, CommandLifetimeHandle},
    plan::{BackupOperationKind, BackupPlan},
    runner::{
        BackupRunnerCommandError, BackupRunnerConfig, BackupRunnerError, BackupRunnerExecutor,
    },
    timestamp::{current_timestamp_marker, state_updated_at},
};

use artifacts::{
    execute_create_snapshot, execute_download_snapshot, execute_finalize_manifest,
    execute_verify_artifact,
};
pub(super) use artifacts::{
    persist_created_snapshot, reconcile_pending_artifact_verification, reconcile_pending_download,
    recorded_snapshot_receipt,
};

pub(super) fn execute_operation_receipt(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
    command_lifetime: Option<CommandLifetimeHandle>,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    match operation.kind {
        BackupOperationKind::Stop => execute_stop(
            executor,
            journal,
            operation,
            required_command_lifetime(operation, command_lifetime)?,
        ),
        BackupOperationKind::CreateSnapshot => execute_create_snapshot(
            executor,
            layout,
            plan,
            journal,
            operation,
            required_command_lifetime(operation, command_lifetime)?,
        ),
        BackupOperationKind::Start => execute_start(
            executor,
            journal,
            operation,
            required_command_lifetime(operation, command_lifetime)?,
        ),
        BackupOperationKind::DownloadSnapshot => execute_download_snapshot(
            executor,
            layout,
            journal,
            operation,
            required_command_lifetime(operation, command_lifetime)?,
        ),
        BackupOperationKind::VerifyArtifact => execute_verify_artifact(layout, journal, operation),
        BackupOperationKind::FinalizeManifest => {
            execute_finalize_manifest(config, layout, plan, journal, operation)
        }
        BackupOperationKind::ValidateTopology
        | BackupOperationKind::ValidateControlAuthority
        | BackupOperationKind::ValidateSnapshotReadAuthority
        | BackupOperationKind::ValidateQuiescencePolicy => {
            Ok(BackupExecutionOperationReceipt::completed(
                journal,
                operation,
                Some(state_updated_at(config.updated_at.as_ref())),
            ))
        }
    }
}

fn execute_stop(
    executor: &mut impl BackupRunnerExecutor,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
    command_lifetime: CommandLifetimeHandle,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    executor
        .stop_canister(&target, command_lifetime)
        .map_err(|error| command_failed(operation.sequence, error))?;
    Ok(BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    ))
}

fn execute_start(
    executor: &mut impl BackupRunnerExecutor,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
    command_lifetime: CommandLifetimeHandle,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    executor
        .start_canister(&target, command_lifetime)
        .map_err(|error| command_failed(operation.sequence, error))?;
    Ok(BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    ))
}

fn required_command_lifetime(
    operation: &BackupExecutionJournalOperation,
    command_lifetime: Option<CommandLifetimeHandle>,
) -> Result<CommandLifetimeHandle, BackupRunnerError> {
    command_lifetime.ok_or_else(|| BackupRunnerError::MissingCommandLifetime {
        sequence: operation.sequence,
        operation_id: operation.operation_id.clone(),
    })
}

pub(super) fn operation_target(
    operation: &BackupExecutionJournalOperation,
) -> Result<String, BackupRunnerError> {
    operation
        .target_canister_id
        .clone()
        .ok_or(BackupRunnerError::MissingOperationTarget {
            sequence: operation.sequence,
        })
}

fn command_failed(sequence: usize, error: BackupRunnerCommandError) -> BackupRunnerError {
    BackupRunnerError::CommandFailed {
        sequence,
        status: error.status,
        message: error.message,
    }
}
