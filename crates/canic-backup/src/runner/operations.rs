use super::{
    BackupRunnerCommandError, BackupRunnerConfig, BackupRunnerError, BackupRunnerExecutor,
    manifest::build_manifest, support::state_updated_at,
};
use crate::{
    artifacts::ArtifactChecksum,
    execution::{
        BackupExecutionJournal, BackupExecutionJournalOperation, BackupExecutionOperationReceipt,
    },
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal, DownloadOperationMetrics},
    persistence::BackupLayout,
    plan::{BackupOperationKind, BackupPlan},
    timestamp::current_timestamp_marker,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) fn execute_operation_receipt(
    config: &BackupRunnerConfig,
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    match operation.kind {
        BackupOperationKind::Stop => execute_stop(executor, journal, operation),
        BackupOperationKind::CreateSnapshot => {
            execute_create_snapshot(executor, layout, plan, journal, operation)
        }
        BackupOperationKind::Start => execute_start(executor, journal, operation),
        BackupOperationKind::DownloadSnapshot => {
            execute_download_snapshot(executor, layout, journal, operation)
        }
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
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    executor
        .stop_canister(&target)
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
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    executor
        .start_canister(&target)
        .map_err(|error| command_failed(operation.sequence, error))?;
    Ok(BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    ))
}

fn execute_create_snapshot(
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    let snapshot_id = executor
        .create_snapshot(&target)
        .map_err(|error| command_failed(operation.sequence, error))?;
    let mut receipt = BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    );
    receipt.snapshot_id = Some(snapshot_id.clone());

    let mut download_journal = read_or_new_download_journal(layout, plan, journal)?;
    upsert_artifact_entry(
        &mut download_journal,
        ArtifactJournalEntry {
            canister_id: target.clone(),
            snapshot_id,
            state: ArtifactState::Created,
            temp_path: None,
            artifact_path: artifact_relative_path(&target),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
            updated_at: current_timestamp_marker(),
        },
    );
    layout.write_journal(&download_journal)?;
    Ok(receipt)
}

fn execute_download_snapshot(
    executor: &mut impl BackupRunnerExecutor,
    layout: &BackupLayout,
    journal: &BackupExecutionJournal,
    operation: &BackupExecutionJournalOperation,
) -> Result<BackupExecutionOperationReceipt, BackupRunnerError> {
    let target = operation_target(operation)?;
    let snapshot_id = snapshot_id_for_target(journal, operation.sequence, &target)?;
    let temp_path = artifact_temp_path(layout.root(), &target);
    if temp_path.exists() {
        fs::remove_dir_all(&temp_path)?;
    }
    fs::create_dir_all(&temp_path)?;
    executor
        .download_snapshot(&target, &snapshot_id, &temp_path)
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

fn execute_verify_artifact(
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

fn execute_finalize_manifest(
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
        let temp_path = download_journal.artifacts[index].temp_path.clone().ok_or(
            BackupRunnerError::MissingArtifactEntry {
                sequence: operation.sequence,
                target_canister_id: canister_id,
            },
        )?;
        let artifact_path = layout
            .root()
            .join(&download_journal.artifacts[index].artifact_path);
        if artifact_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("artifact path already exists: {}", artifact_path.display()),
            )
            .into());
        }
        fs::rename(&temp_path, artifact_path)?;
        download_journal.artifacts[index].temp_path = None;
        download_journal.artifacts[index]
            .advance_to(ArtifactState::Durable, current_timestamp_marker())?;
        layout.write_journal(&download_journal)?;
    }

    let manifest = build_manifest(config, plan, &download_journal)?;
    layout.write_manifest(&manifest)?;
    Ok(BackupExecutionOperationReceipt::completed(
        journal,
        operation,
        Some(current_timestamp_marker()),
    ))
}

fn read_or_new_download_journal(
    layout: &BackupLayout,
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
) -> Result<DownloadJournal, BackupRunnerError> {
    if layout.journal_path().is_file() {
        let mut journal = layout.read_journal()?;
        journal.discovery_topology_hash = Some(plan.topology_hash_before_quiesce.clone());
        journal.pre_snapshot_topology_hash = Some(plan.topology_hash_before_quiesce.clone());
        return Ok(journal);
    }

    Ok(DownloadJournal {
        journal_version: 1,
        backup_id: journal.run_id.clone(),
        discovery_topology_hash: Some(plan.topology_hash_before_quiesce.clone()),
        pre_snapshot_topology_hash: Some(plan.topology_hash_before_quiesce.clone()),
        operation_metrics: DownloadOperationMetrics::default(),
        artifacts: Vec::new(),
    })
}

fn upsert_artifact_entry(journal: &mut DownloadJournal, entry: ArtifactJournalEntry) {
    if let Some(existing) = journal
        .artifacts
        .iter_mut()
        .find(|existing| existing.canister_id == entry.canister_id)
    {
        *existing = entry;
    } else {
        journal.operation_metrics.target_count = journal.artifacts.len() + 1;
        journal.artifacts.push(entry);
    }
}

fn artifact_entry_mut<'a>(
    journal: &'a mut DownloadJournal,
    sequence: usize,
    target: &str,
) -> Result<&'a mut ArtifactJournalEntry, BackupRunnerError> {
    journal
        .artifacts
        .iter_mut()
        .find(|entry| entry.canister_id == target)
        .ok_or_else(|| BackupRunnerError::MissingArtifactEntry {
            sequence,
            target_canister_id: target.to_string(),
        })
}

fn snapshot_id_for_target(
    journal: &BackupExecutionJournal,
    sequence: usize,
    target: &str,
) -> Result<String, BackupRunnerError> {
    journal
        .operation_receipts
        .iter()
        .rev()
        .find(|receipt| {
            receipt.kind == BackupOperationKind::CreateSnapshot
                && receipt.target_canister_id.as_deref() == Some(target)
                && receipt.snapshot_id.is_some()
        })
        .and_then(|receipt| receipt.snapshot_id.clone())
        .ok_or_else(|| BackupRunnerError::MissingSnapshotId {
            sequence,
            target_canister_id: target.to_string(),
        })
}

fn operation_target(
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

fn artifact_relative_path(canister_id: &str) -> String {
    safe_path_segment(canister_id)
}

fn artifact_temp_path(root: &Path, canister_id: &str) -> PathBuf {
    root.join(format!("{}.tmp", safe_path_segment(canister_id)))
}

fn safe_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}
