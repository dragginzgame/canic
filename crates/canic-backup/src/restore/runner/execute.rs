//! Module: restore::runner::execute
//!
//! Responsibility: execute ready restore apply journal operations.
//! Does not own: command rendering, apply journal validation, or restore planning.
//! Boundary: claims operations, invokes an injected executor, and persists receipts.

use super::{
    RestoreApplyCommandOutputPair, RestoreApplyJournal, RestoreApplyOperationKind,
    RestoreApplyOperationReceipt, RestoreApplyRunnerCommand,
    artifact::{cleanup_upload_staging, stage_upload_artifact},
    constants::{
        RESTORE_RUN_COMMAND_EXIT_PREFIX, RESTORE_RUN_MISSING_UPLOADED_SNAPSHOT_ID,
        RESTORE_RUN_OUTPUT_RECEIPT_LIMIT, RESTORE_RUN_STOPPED_PRECONDITION_FAILED,
        RESTORE_RUN_VERIFICATION_EVIDENCE_MISMATCH,
    },
    io::{read_apply_journal_file, write_apply_journal_file},
    precondition::{
        RestoreObservedCanisterStatus, enforce_stopped_canister_precondition,
        observe_canister_status, parse_canister_status_evidence,
    },
    status::{
        enforce_restore_run_command_available, enforce_restore_run_executable,
        parse_uploaded_snapshot_id, restore_command_unavailable_error,
        restore_run_max_steps_reached, restore_run_next_action, restore_run_stopped_reason,
    },
    types::{
        RestoreReconciledCommandSuccess, RestoreRunExecutedOperation, RestoreRunOperationReceipt,
        RestoreRunPreparedOperation, RestoreRunResponse, RestoreRunResponseMode,
        RestoreRunStepOutcome, RestoreRunnerCommandExecutor, RestoreRunnerConfig,
        RestoreRunnerError, RestoreRunnerOutcome, RestoreStoppedPreconditionFailure,
    },
};
use crate::{
    persistence::{CommandLifetimeLock, CommandLifetimeLockError, JournalLock},
    timestamp::state_updated_at,
};
use std::{collections::BTreeSet, path::Path};

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

pub fn restore_run_execute_result_with_executor(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
) -> Result<RestoreRunnerOutcome, RestoreRunnerError> {
    restore_run_execute_result_with_terminal_writer(config, executor, &mut |path, journal| {
        write_apply_journal_file(path, journal)
    })
}

#[cfg(all(test, unix))]
pub fn restore_run_execute_with_terminal_barriers(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    mut barriers: impl FnMut(crate::persistence::DurableWriteBarrier),
) -> Result<RestoreRunResponse, RestoreRunnerError> {
    let run =
        restore_run_execute_result_with_terminal_writer(config, executor, &mut |path, journal| {
            super::io::write_apply_journal_file_at_barriers(path, journal, &mut barriers)
        })?;
    if let Some(error) = run.error {
        return Err(error);
    }
    Ok(run.response)
}

fn restore_run_execute_result_with_terminal_writer(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    terminal_writer: &mut impl FnMut(&Path, &RestoreApplyJournal) -> Result<(), RestoreRunnerError>,
) -> Result<RestoreRunnerOutcome, RestoreRunnerError> {
    let _lock = JournalLock::acquire(&config.journal)?;
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
        let prepared = restore_run_prepare_next_operation(config, executor, &mut journal)?;
        let sequence = prepared.sequence;
        match restore_run_execute_prepared_operation(
            config,
            executor,
            &mut journal,
            prepared,
            terminal_writer,
        )? {
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

fn restore_run_prepare_next_operation(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    journal: &mut RestoreApplyJournal,
) -> Result<RestoreRunPreparedOperation, RestoreRunnerError> {
    let preview = journal.next_command_preview_with_config(&config.command);
    enforce_restore_run_command_available(&preview)?;

    let operation = preview
        .operation
        .clone()
        .ok_or_else(|| restore_command_unavailable_error(&preview))?;
    let mut command = preview
        .command
        .clone()
        .ok_or_else(|| restore_command_unavailable_error(&preview))?;
    let sequence = operation.sequence;
    let command_lock = restore_command_lock(config, &operation)?;
    let attempt = journal
        .operation_receipts
        .iter()
        .filter(|receipt| receipt.sequence == sequence)
        .count()
        + 1;
    let reconciled_success = reconcile_restore_operation(
        config,
        executor,
        &operation,
        command_lock.as_ref().map(CommandLifetimeLock::handle),
    )?;
    if let Some(reconciled_success) = reconciled_success {
        return Ok(RestoreRunPreparedOperation {
            operation,
            command: reconciled_success.command.clone(),
            sequence,
            attempt,
            command_lock,
            _staged_artifact: None,
            reconciled_success: Some(reconciled_success),
        });
    }
    let staged_artifact = stage_upload_artifact(config, journal, &operation)?;
    if let Some(staged) = &staged_artifact {
        command = crate::restore::RestoreApplyRunnerCommand::from_operation_with_artifact_path(
            &operation,
            journal,
            &config.command,
            Some(staged.artifact_path()),
        )
        .ok_or_else(|| restore_command_unavailable_error(&preview))?;
    }
    let upload_inventory_before = if operation.operation
        == RestoreApplyOperationKind::UploadSnapshot
        && operation.snapshot_ids_before.is_none()
    {
        Some(
            observe_snapshot_inventory(
                config,
                executor,
                &operation,
                command_lock.as_ref().map(CommandLifetimeLock::handle),
            )?
            .snapshot_ids,
        )
    } else {
        None
    };

    enforce_apply_claim_sequence(sequence, journal)?;
    let pending_at = Some(state_updated_at(config.updated_at.as_ref()));
    if operation.operation == RestoreApplyOperationKind::UploadSnapshot {
        let snapshot_ids_before = operation
            .snapshot_ids_before
            .clone()
            .or(upload_inventory_before)
            .ok_or(RestoreRunnerError::InvalidSnapshotInventory { sequence })?;
        journal.mark_upload_snapshot_pending_at(sequence, pending_at, snapshot_ids_before)?;
    } else {
        journal.mark_operation_pending_at(sequence, pending_at)?;
    }
    write_apply_journal_file(&config.journal, journal)?;

    Ok(RestoreRunPreparedOperation {
        operation,
        command,
        sequence,
        attempt,
        command_lock,
        _staged_artifact: staged_artifact,
        reconciled_success: None,
    })
}

fn restore_run_execute_prepared_operation(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    journal: &mut RestoreApplyJournal,
    mut prepared: RestoreRunPreparedOperation,
    terminal_writer: &mut impl FnMut(&Path, &RestoreApplyJournal) -> Result<(), RestoreRunnerError>,
) -> Result<RestoreRunStepOutcome, RestoreRunnerError> {
    if prepared.reconciled_success.is_some() {
        return restore_run_commit_reconciled_success(config, journal, prepared, terminal_writer);
    }
    let command_lifetime = prepared
        .command_lock
        .as_ref()
        .map(CommandLifetimeLock::handle);
    if prepared.command.requires_stopped_canister {
        let precondition = enforce_stopped_canister_precondition(
            config,
            executor,
            &prepared.operation,
            prepared.attempt,
            config.updated_at.as_ref(),
            command_lifetime,
        );
        match precondition {
            Ok(Some(outcome)) => {
                finish_restore_command_lock(&mut prepared)?;
                return restore_run_commit_precondition_failure(
                    config,
                    journal,
                    prepared,
                    outcome,
                    terminal_writer,
                );
            }
            Ok(None) => {}
            Err(error) => {
                finish_restore_command_lock(&mut prepared)?;
                return Err(error);
            }
        }
    }

    let output = executor.execute(&prepared.command, command_lifetime);
    finish_restore_command_lock(&mut prepared)?;
    let output = output?;
    cleanup_upload_staging(config, &prepared.operation)?;
    let status_label = output.status;
    let output_pair = RestoreApplyCommandOutputPair::from_bytes(
        &output.stdout,
        &output.stderr,
        RESTORE_RUN_OUTPUT_RECEIPT_LIMIT,
    );

    if output.success {
        if matches!(
            prepared.operation.operation,
            RestoreApplyOperationKind::VerifyMember | RestoreApplyOperationKind::VerifyDeployment
        ) && !verification_output_matches(&output_pair, &prepared.operation)
        {
            return restore_run_commit_command_failure(
                config,
                journal,
                prepared,
                RESTORE_RUN_VERIFICATION_EVIDENCE_MISMATCH.to_string(),
                output_pair,
                terminal_writer,
            );
        }
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
                terminal_writer,
            );
        }

        return restore_run_commit_command_success(
            config,
            journal,
            prepared,
            status_label,
            output_pair,
            uploaded_snapshot_id,
            terminal_writer,
        );
    }

    restore_run_commit_command_failure(
        config,
        journal,
        prepared,
        status_label,
        output_pair,
        terminal_writer,
    )
}

fn restore_run_commit_reconciled_success(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
    mut prepared: RestoreRunPreparedOperation,
    terminal_writer: &mut impl FnMut(&Path, &RestoreApplyJournal) -> Result<(), RestoreRunnerError>,
) -> Result<RestoreRunStepOutcome, RestoreRunnerError> {
    let reconciled = prepared
        .reconciled_success
        .take()
        .expect("reconciled restore operation");
    prepared.command = reconciled.command;
    cleanup_upload_staging(config, &prepared.operation)?;
    finish_restore_command_lock(&mut prepared)?;
    restore_run_commit_command_success(
        config,
        journal,
        prepared,
        reconciled.status,
        reconciled.output,
        reconciled.uploaded_snapshot_id,
        terminal_writer,
    )
}

fn verification_output_matches(
    output: &RestoreApplyCommandOutputPair,
    operation: &crate::restore::RestoreApplyJournalOperation,
) -> bool {
    verification_json_matches(
        &output.stdout.text,
        operation.expected_module_hash.as_deref(),
    ) || verification_json_matches(
        &output.stderr.text,
        operation.expected_module_hash.as_deref(),
    )
}

fn verification_json_matches(output: &str, expected_module_hash: Option<&str>) -> bool {
    let Some(evidence) = parse_canister_status_evidence(output) else {
        return false;
    };
    if evidence.status != RestoreObservedCanisterStatus::Running {
        return false;
    }
    let Some(expected_module_hash) = expected_module_hash else {
        return true;
    };
    evidence.module_hash.as_deref().is_some_and(|actual| {
        normalize_module_hash(actual) == normalize_module_hash(expected_module_hash)
    })
}

fn normalize_module_hash(module_hash: &str) -> String {
    module_hash
        .trim()
        .strip_prefix("0x")
        .unwrap_or_else(|| module_hash.trim())
        .to_ascii_lowercase()
}

#[cfg(test)]
mod verification_tests {
    use super::*;

    #[test]
    fn verification_requires_running_status_and_matching_module_hash() {
        assert!(verification_json_matches(
            r#"{"status":"Running","module_hash":"0xABCD"}"#,
            Some("abcd")
        ));
        assert!(!verification_json_matches(
            r#"{"status":"Running","module_hash":"0xDEAD"}"#,
            Some("abcd")
        ));
        assert!(!verification_json_matches(
            r#"{"status":"Stopped","module_hash":"0xABCD"}"#,
            Some("abcd")
        ));
        assert!(!verification_json_matches("not json", Some("abcd")));
    }
}

fn reconcile_restore_operation(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    operation: &crate::restore::RestoreApplyJournalOperation,
    command_lifetime: Option<crate::persistence::CommandLifetimeHandle>,
) -> Result<Option<RestoreReconciledCommandSuccess>, RestoreRunnerError> {
    let recovering = operation.state == crate::restore::RestoreApplyOperationState::Pending
        || (operation.operation == RestoreApplyOperationKind::UploadSnapshot
            && operation.snapshot_ids_before.is_some());
    if !recovering {
        return Ok(None);
    }

    match operation.operation {
        RestoreApplyOperationKind::StopCanister => reconcile_lifecycle_operation(
            config,
            executor,
            operation,
            command_lifetime,
            RestoreObservedCanisterStatus::Stopped,
        ),
        RestoreApplyOperationKind::StartCanister => reconcile_lifecycle_operation(
            config,
            executor,
            operation,
            command_lifetime,
            RestoreObservedCanisterStatus::Running,
        ),
        RestoreApplyOperationKind::UploadSnapshot => {
            reconcile_upload_operation(config, executor, operation, command_lifetime)
        }
        RestoreApplyOperationKind::LoadSnapshot
        | RestoreApplyOperationKind::VerifyMember
        | RestoreApplyOperationKind::VerifyDeployment => Ok(None),
    }
}

fn reconcile_lifecycle_operation(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    operation: &crate::restore::RestoreApplyJournalOperation,
    command_lifetime: Option<crate::persistence::CommandLifetimeHandle>,
    completed_status: RestoreObservedCanisterStatus,
) -> Result<Option<RestoreReconciledCommandSuccess>, RestoreRunnerError> {
    let observation = observe_canister_status(config, executor, operation, command_lifetime)?;
    if !observation.success {
        return Err(RestoreRunnerError::CanisterStatusObservationFailed {
            sequence: operation.sequence,
            status: observation.status_label,
        });
    }
    let status = observation
        .status
        .ok_or(RestoreRunnerError::CanisterStatusUnknown {
            sequence: operation.sequence,
        })?;
    if status == RestoreObservedCanisterStatus::Stopping {
        return Err(RestoreRunnerError::CanisterStatusUnsettled {
            sequence: operation.sequence,
        });
    }
    if status != completed_status {
        return Ok(None);
    }

    Ok(Some(RestoreReconciledCommandSuccess {
        command: observation.command,
        status: observation.status_label,
        output: observation.output,
        uploaded_snapshot_id: None,
    }))
}

fn reconcile_upload_operation(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    operation: &crate::restore::RestoreApplyJournalOperation,
    command_lifetime: Option<crate::persistence::CommandLifetimeHandle>,
) -> Result<Option<RestoreReconciledCommandSuccess>, RestoreRunnerError> {
    let baseline = operation.snapshot_ids_before.as_ref().ok_or(
        RestoreRunnerError::InvalidSnapshotInventory {
            sequence: operation.sequence,
        },
    )?;
    let observation = observe_snapshot_inventory(config, executor, operation, command_lifetime)?;
    let baseline = baseline.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let observed = observation
        .snapshot_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let missing = baseline
        .difference(&observed)
        .map(|snapshot_id| (*snapshot_id).to_string())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(RestoreRunnerError::SnapshotInventoryLostBaseline {
            sequence: operation.sequence,
            snapshot_ids: missing,
        });
    }
    let created = observed
        .difference(&baseline)
        .map(|snapshot_id| (*snapshot_id).to_string())
        .collect::<Vec<_>>();
    match created.as_slice() {
        [] => Ok(None),
        [snapshot_id] => Ok(Some(RestoreReconciledCommandSuccess {
            command: observation.command,
            status: observation.status,
            output: observation.output,
            uploaded_snapshot_id: Some(snapshot_id.clone()),
        })),
        _ => Err(RestoreRunnerError::UploadedSnapshotIdentityAmbiguous {
            sequence: operation.sequence,
            snapshot_ids: created,
        }),
    }
}

struct RestoreSnapshotInventoryObservation {
    command: RestoreApplyRunnerCommand,
    status: String,
    output: RestoreApplyCommandOutputPair,
    snapshot_ids: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreSnapshotInventoryResponse {
    snapshots: Vec<RestoreSnapshotInventoryEntry>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreSnapshotInventoryEntry {
    snapshot_id: String,
    #[serde(rename = "taken_at_timestamp")]
    _taken_at_timestamp: Option<u64>,
    #[serde(rename = "total_size_bytes")]
    _total_size_bytes: Option<u64>,
}

fn observe_snapshot_inventory(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    operation: &crate::restore::RestoreApplyJournalOperation,
    command_lifetime: Option<crate::persistence::CommandLifetimeHandle>,
) -> Result<RestoreSnapshotInventoryObservation, RestoreRunnerError> {
    let mut args = vec![
        "canister".to_string(),
        "snapshot".to_string(),
        "list".to_string(),
        operation.target_canister.clone(),
        "--json".to_string(),
    ];
    if let Some(environment) = &config.command.environment {
        args.push("-e".to_string());
        args.push(environment.clone());
    }
    let command = RestoreApplyRunnerCommand {
        program: config.command.program.clone(),
        args,
        mutates: false,
        requires_stopped_canister: false,
        note: "observes target snapshot inventory for upload reconciliation".to_string(),
    };
    let raw = executor.execute(&command, command_lifetime)?;
    let output = RestoreApplyCommandOutputPair::from_bytes(
        &raw.stdout,
        &raw.stderr,
        RESTORE_RUN_OUTPUT_RECEIPT_LIMIT,
    );
    if !raw.success {
        return Err(RestoreRunnerError::SnapshotInventoryObservationFailed {
            sequence: operation.sequence,
            status: raw.status,
        });
    }
    let inventory = serde_json::from_slice::<RestoreSnapshotInventoryResponse>(&raw.stdout)
        .map_err(|_| RestoreRunnerError::InvalidSnapshotInventory {
            sequence: operation.sequence,
        })?;
    let mut identities = BTreeSet::new();
    for snapshot in inventory.snapshots {
        let snapshot_id = (!snapshot.snapshot_id.trim().is_empty())
            .then(|| snapshot.snapshot_id.trim())
            .ok_or(RestoreRunnerError::InvalidSnapshotInventory {
                sequence: operation.sequence,
            })?;
        if !identities.insert(snapshot_id.to_string()) {
            return Err(RestoreRunnerError::InvalidSnapshotInventory {
                sequence: operation.sequence,
            });
        }
    }
    Ok(RestoreSnapshotInventoryObservation {
        command,
        status: raw.status,
        output,
        snapshot_ids: identities.into_iter().collect(),
    })
}

fn restore_command_lock(
    config: &RestoreRunnerConfig,
    operation: &crate::restore::RestoreApplyJournalOperation,
) -> Result<Option<CommandLifetimeLock>, RestoreRunnerError> {
    if !matches!(
        operation.operation,
        RestoreApplyOperationKind::UploadSnapshot
            | RestoreApplyOperationKind::StopCanister
            | RestoreApplyOperationKind::LoadSnapshot
            | RestoreApplyOperationKind::StartCanister
    ) {
        return Ok(None);
    }

    CommandLifetimeLock::acquire(&config.journal, operation.sequence)
        .map(Some)
        .map_err(|error| restore_command_lock_error(operation, error))
}

fn finish_restore_command_lock(
    prepared: &mut RestoreRunPreparedOperation,
) -> Result<(), RestoreRunnerError> {
    let Some(command_lock) = prepared.command_lock.take() else {
        return Ok(());
    };
    command_lock
        .finish()
        .map_err(|error| restore_command_lock_error(&prepared.operation, error))
}

fn restore_command_lock_error(
    operation: &crate::restore::RestoreApplyJournalOperation,
    error: CommandLifetimeLockError,
) -> RestoreRunnerError {
    match error {
        CommandLifetimeLockError::InFlight { lock_path } => RestoreRunnerError::CommandInFlight {
            sequence: operation.sequence,
            operation: operation.operation.clone(),
            lock_path,
        },
        CommandLifetimeLockError::UnsafeEntry { lock_path, kind } => {
            RestoreRunnerError::CommandLockUnsafeEntry {
                sequence: operation.sequence,
                operation: operation.operation.clone(),
                lock_path,
                kind,
            }
        }
        CommandLifetimeLockError::Io(error) => RestoreRunnerError::Io(error),
    }
}

fn restore_run_commit_missing_uploaded_snapshot_id(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
    prepared: RestoreRunPreparedOperation,
    output_pair: RestoreApplyCommandOutputPair,
    terminal_writer: &mut impl FnMut(&Path, &RestoreApplyJournal) -> Result<(), RestoreRunnerError>,
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
    terminal_writer(&config.journal, journal)?;

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

fn restore_run_commit_precondition_failure(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
    prepared: RestoreRunPreparedOperation,
    outcome: RestoreStoppedPreconditionFailure,
    terminal_writer: &mut impl FnMut(&Path, &RestoreApplyJournal) -> Result<(), RestoreRunnerError>,
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
    terminal_writer(&config.journal, journal)?;

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

fn restore_run_commit_command_success(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
    prepared: RestoreRunPreparedOperation,
    status_label: String,
    output_pair: RestoreApplyCommandOutputPair,
    uploaded_snapshot_id: Option<String>,
    terminal_writer: &mut impl FnMut(&Path, &RestoreApplyJournal) -> Result<(), RestoreRunnerError>,
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
    terminal_writer(&config.journal, journal)?;

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

fn restore_run_commit_command_failure(
    config: &RestoreRunnerConfig,
    journal: &mut RestoreApplyJournal,
    prepared: RestoreRunPreparedOperation,
    status_label: String,
    output_pair: RestoreApplyCommandOutputPair,
    terminal_writer: &mut impl FnMut(&Path, &RestoreApplyJournal) -> Result<(), RestoreRunnerError>,
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
    terminal_writer(&config.journal, journal)?;

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
