//! Module: runner::operations::journal
//!
//! Responsibility: manage download journal entries during runner operation execution.
//! Does not own: persistence layout, artifact verification, or execution journal validation.
//! Boundary: provides scoped helpers for operation execution modules.

use crate::{
    execution::BackupExecutionJournal,
    journal::{ArtifactJournalEntry, DownloadJournal, DownloadOperationMetrics},
    persistence::BackupLayout,
    plan::{BackupOperationKind, BackupPlan},
    runner::BackupRunnerError,
};

pub(super) fn read_or_new_download_journal(
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

pub(super) fn upsert_artifact_entry(journal: &mut DownloadJournal, entry: ArtifactJournalEntry) {
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

pub(super) fn artifact_entry_mut<'a>(
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

pub(super) fn snapshot_id_for_target(
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
