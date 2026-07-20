//! Module: runner::operations::artifacts
//!
//! Responsibility: execute snapshot, download, checksum, and manifest artifact steps.
//! Does not own: executor command implementations, plan construction, or layout validation.
//! Boundary: mutates journals through persistence APIs and returns operation receipts.

use crate::{
    artifacts::ArtifactChecksum,
    execution::{
        BackupExecutionJournal, BackupExecutionJournalOperation, BackupExecutionOperationReceipt,
    },
    journal::{ArtifactJournalEntry, ArtifactState},
    persistence::{
        BackupLayout, CommandLifetimeHandle, PersistenceError, commit_artifact_directory,
    },
    plan::BackupPlan,
    runner::{
        BackupRunnerConfig, BackupRunnerError, BackupRunnerExecutor, BackupRunnerSnapshot,
        manifest::build_manifest,
        operations::{
            command_failed,
            journal::{
                artifact_entry_mut, read_or_new_download_journal, snapshot_id_for_target,
                upsert_artifact_entry,
            },
            operation_target,
            path::{artifact_relative_path, artifact_temp_path, ensure_expected_temp_path},
        },
    },
    timestamp::current_timestamp_marker,
};

use std::{fs, path::Path};

pub(super) fn execute_create_snapshot(
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
    command_lifetime: CommandLifetimeHandle,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    if let Some(receipt) = recorded_snapshot_receipt(layout, plan, journal, operation, &target)? {
        return Ok(receipt);
    }
    let snapshot = executor
        .create_snapshot(&target, command_lifetime)
        .map_err(|error| command_failed(operation.sequence, error))?;
    persist_created_snapshot(layout, plan, journal, operation, &target, snapshot)
}

pub(in crate::runner) fn recorded_snapshot_receipt(
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
    target: &str,
) -> Result<Option<BackupExecutionOperationReceipt>, BackupRunnerError> {
    let download_journal = read_or_new_download_journal(layout, plan, journal)?;
    let matching = download_journal
        .artifacts
        .iter()
        .filter(|artifact| artifact.canister_id == target)
        .collect::<Vec<_>>();
    match matching.as_slice() {
        [] => Ok(None),
        [artifact] => Ok(Some(snapshot_receipt(
            journal,
            operation,
            BackupRunnerSnapshot {
                snapshot_id: artifact.snapshot_id.clone(),
                taken_at_timestamp: artifact.snapshot_taken_at_timestamp,
                total_size_bytes: artifact.snapshot_total_size_bytes,
            },
        ))),
        artifacts => Err(BackupRunnerError::AmbiguousArtifactSnapshot {
            sequence: operation.sequence,
            target_canister_id: target.to_string(),
            snapshot_ids: artifacts
                .iter()
                .map(|artifact| artifact.snapshot_id.clone())
                .collect(),
        }),
    }
}

pub(in crate::runner) fn persist_created_snapshot(
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
    target: &str,
    snapshot: BackupRunnerSnapshot,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let mut download_journal = read_or_new_download_journal(layout, plan, journal)?;
    let receipt = snapshot_receipt(journal, operation, snapshot.clone());

    upsert_artifact_entry(
        &mut download_journal,
        ArtifactJournalEntry {
            canister_id: target.to_string(),
            snapshot_id: snapshot.snapshot_id,
            snapshot_taken_at_timestamp: snapshot.taken_at_timestamp,
            snapshot_total_size_bytes: snapshot.total_size_bytes,
            state: ArtifactState::Created,
            temp_path: None,
            artifact_path: artifact_relative_path(target),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
            updated_at: current_timestamp_marker(),
        },
    );
    layout.write_journal(&download_journal)?;
    Ok(receipt)
}

fn snapshot_receipt(
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
    snapshot: BackupRunnerSnapshot,
) -> BackupExecutionOperationReceipt {
    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    );
    receipt.snapshot_id = Some(snapshot.snapshot_id.clone());
    receipt.snapshot_taken_at_timestamp = snapshot.taken_at_timestamp;
    receipt.snapshot_total_size_bytes = snapshot.total_size_bytes;
    receipt
}

pub(super) fn execute_download_snapshot(
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
    command_lifetime: CommandLifetimeHandle,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    let snapshot_id = snapshot_id_for_target(journal, operation.sequence, &target)?;
    let temp_path = artifact_temp_path(layout.root(), &target);
    if temp_path.exists() {
        fs::remove_dir_all(&temp_path)?;
    }
    fs::create_dir_all(&temp_path)?;
    executor
        .download_snapshot(&target, &snapshot_id, &temp_path, command_lifetime)
        .map_err(|error| command_failed(operation.sequence, error))?;

    let mut download_journal = layout.read_journal()?;
    let entry = artifact_entry_mut(&mut download_journal, operation.sequence, &target)?;
    entry.temp_path = Some(temp_path.display().to_string());
    entry.advance_to(ArtifactState::Downloaded, current_timestamp_marker())?;
    layout.write_journal(&download_journal)?;

    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    );
    receipt.artifact_path = Some(artifact_relative_path(&target));
    Ok(receipt)
}

pub(super) fn execute_verify_artifact(
    layout: &BackupLayout,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    let mut download_journal = layout.read_journal()?;
    let entry = artifact_entry_mut(&mut download_journal, operation.sequence, &target)?;
    let temp_path =
        entry
            .temp_path
            .as_deref()
            .ok_or_else(|| BackupRunnerError::MissingArtifactEntry {
                sequence: operation.sequence,
                target_canister_id: target.clone(),
            })?;
    ensure_expected_temp_path(layout, operation.sequence, &target, temp_path)?;
    let checksum = ArtifactChecksum::from_path(Path::new(temp_path))?;
    entry.checksum = Some(checksum.hash.clone());
    entry.advance_to(ArtifactState::ChecksumVerified, current_timestamp_marker())?;
    layout.write_journal(&download_journal)?;

    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    );
    receipt.checksum = Some(checksum.hash);
    Ok(receipt)
}

pub(super) fn execute_finalize_manifest(
    config: &BackupRunnerConfig,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let mut download_journal = layout.read_journal()?;
    for index in 0..download_journal.artifacts.len() {
        if download_journal.artifacts[index].state == ArtifactState::Durable {
            continue;
        }
        let canister_id = download_journal.artifacts[index].canister_id.clone();
        let snapshot_id = download_journal.artifacts[index].snapshot_id.clone();
        if download_journal.artifacts[index].state != ArtifactState::ChecksumVerified {
            return Err(PersistenceError::ArtifactNotChecksumVerified {
                canister_id,
                snapshot_id,
                state: download_journal.artifacts[index].state,
            }
            .into());
        }
        let temp_path = download_journal.artifacts[index]
            .temp_path
            .clone()
            .ok_or_else(|| BackupRunnerError::MissingArtifactEntry {
                sequence: operation.sequence,
                target_canister_id: canister_id.clone(),
            })?;
        ensure_expected_temp_path(layout, operation.sequence, &canister_id, &temp_path)?;
        let artifact_path = layout
            .root()
            .join(&download_journal.artifacts[index].artifact_path);
        let checksum = download_journal.artifacts[index]
            .checksum
            .as_deref()
            .ok_or_else(|| PersistenceError::MissingJournalArtifactChecksum {
                canister_id: download_journal.artifacts[index].canister_id.clone(),
                snapshot_id: download_journal.artifacts[index].snapshot_id.clone(),
            })?;
        commit_artifact_directory(Path::new(&temp_path), &artifact_path, checksum)?;

        let mut completed_journal = download_journal.clone();
        completed_journal.artifacts[index].temp_path = None;
        completed_journal.artifacts[index]
            .advance_to(ArtifactState::Durable, current_timestamp_marker())?;
        layout.write_journal(&completed_journal)?;
        download_journal = completed_journal;
    }

    let manifest = build_manifest(config, plan, &download_journal)?;
    layout.write_manifest(&manifest)?;
    Ok(BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    ))
}
