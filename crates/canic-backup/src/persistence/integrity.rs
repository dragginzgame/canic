use super::{BackupLayout, PersistenceError};
use crate::{
    artifacts::ArtifactChecksum,
    execution::{
        BackupExecutionJournal, BackupExecutionOperationReceiptOutcome,
        BackupExecutionOperationState,
    },
    journal::{ArtifactState, DownloadJournal},
    manifest::{FleetBackupManifest, FleetMember},
    plan::{BackupOperationKind, BackupPlan},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

///
/// BackupIntegrityReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupIntegrityReport {
    pub backup_id: String,
    pub verified: bool,
    pub manifest_members: usize,
    pub journal_artifacts: usize,
    pub durable_artifacts: usize,
    pub artifacts: Vec<ArtifactIntegrityReport>,
}

///
/// BackupExecutionIntegrityReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupExecutionIntegrityReport {
    pub plan_id: String,
    pub run_id: String,
    pub verified: bool,
    pub plan_operations: usize,
    pub journal_operations: usize,
}

///
/// ArtifactIntegrityReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactIntegrityReport {
    pub canister_id: String,
    pub snapshot_id: String,
    pub artifact_path: String,
    pub checksum: String,
}

///
/// TopologyReceiptMismatch
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct TopologyReceiptMismatch {
    field: String,
    manifest: String,
    journal: Option<String>,
}

// Verify cross-file backup layout consistency and artifact checksums.
pub(super) fn verify_layout_integrity(
    layout: &BackupLayout,
    manifest: &FleetBackupManifest,
    journal: &DownloadJournal,
) -> Result<BackupIntegrityReport, PersistenceError> {
    verify_manifest_journal_binding(manifest, journal)?;

    let expected_artifacts = expected_artifact_keys(manifest);
    for entry in &journal.artifacts {
        if !expected_artifacts.contains(&(entry.canister_id.as_str(), entry.snapshot_id.as_str())) {
            return Err(PersistenceError::UnexpectedJournalArtifact {
                canister_id: entry.canister_id.clone(),
                snapshot_id: entry.snapshot_id.clone(),
            });
        }
    }

    let mut artifacts = Vec::with_capacity(journal.artifacts.len());
    for member in &manifest.fleet.members {
        artifacts.push(verify_member_artifact(layout, journal, member)?);
    }

    Ok(BackupIntegrityReport {
        backup_id: manifest.backup_id.clone(),
        verified: true,
        manifest_members: manifest.fleet.members.len(),
        journal_artifacts: journal.artifacts.len(),
        durable_artifacts: artifacts.len(),
        artifacts,
    })
}

fn verify_manifest_journal_binding(
    manifest: &FleetBackupManifest,
    journal: &DownloadJournal,
) -> Result<(), PersistenceError> {
    if manifest.backup_id != journal.backup_id {
        return Err(PersistenceError::BackupIdMismatch {
            manifest: manifest.backup_id.clone(),
            journal: journal.backup_id.clone(),
        });
    }

    if let Some(mismatch) = topology_receipt_mismatches(manifest, journal)
        .into_iter()
        .next()
    {
        return Err(PersistenceError::ManifestJournalTopologyReceiptMismatch {
            field: mismatch.field,
            manifest: mismatch.manifest,
            journal: mismatch.journal,
        });
    }

    Ok(())
}

fn expected_artifact_keys(manifest: &FleetBackupManifest) -> BTreeSet<(&str, &str)> {
    manifest
        .fleet
        .members
        .iter()
        .map(|member| {
            (
                member.canister_id.as_str(),
                member.source_snapshot.snapshot_id.as_str(),
            )
        })
        .collect()
}

fn verify_member_artifact(
    layout: &BackupLayout,
    journal: &DownloadJournal,
    member: &FleetMember,
) -> Result<ArtifactIntegrityReport, PersistenceError> {
    let Some(entry) = journal.artifacts.iter().find(|entry| {
        entry.canister_id == member.canister_id
            && entry.snapshot_id == member.source_snapshot.snapshot_id
    }) else {
        return Err(PersistenceError::MissingJournalArtifact {
            canister_id: member.canister_id.clone(),
            snapshot_id: member.source_snapshot.snapshot_id.clone(),
        });
    };

    if entry.state != ArtifactState::Durable {
        return Err(PersistenceError::NonDurableArtifact {
            canister_id: entry.canister_id.clone(),
            snapshot_id: entry.snapshot_id.clone(),
        });
    }

    let expected_hash = entry.checksum.as_deref().ok_or_else(|| {
        PersistenceError::MissingJournalArtifactChecksum {
            canister_id: entry.canister_id.clone(),
            snapshot_id: entry.snapshot_id.clone(),
        }
    })?;
    validate_member_artifact_metadata(member, entry, expected_hash)?;
    let artifact_path = resolve_backup_artifact_path(layout.root(), &entry.artifact_path)
        .ok_or_else(|| PersistenceError::ArtifactPathEscapesBackup {
            artifact_path: entry.artifact_path.clone(),
        })?;
    if !artifact_path.exists() {
        return Err(PersistenceError::MissingArtifact(
            artifact_path.display().to_string(),
        ));
    }

    ArtifactChecksum::from_path(&artifact_path)?.verify(expected_hash)?;
    Ok(ArtifactIntegrityReport {
        canister_id: entry.canister_id.clone(),
        snapshot_id: entry.snapshot_id.clone(),
        artifact_path: artifact_path.display().to_string(),
        checksum: expected_hash.to_string(),
    })
}

fn validate_member_artifact_metadata(
    member: &FleetMember,
    entry: &crate::journal::ArtifactJournalEntry,
    expected_hash: &str,
) -> Result<(), PersistenceError> {
    if member.source_snapshot.artifact_path != entry.artifact_path {
        return Err(PersistenceError::ManifestJournalArtifactPathMismatch {
            canister_id: entry.canister_id.clone(),
            snapshot_id: entry.snapshot_id.clone(),
            manifest: member.source_snapshot.artifact_path.clone(),
            journal: entry.artifact_path.clone(),
        });
    }
    if let Some(manifest_hash) = member.source_snapshot.checksum.as_deref()
        && manifest_hash != expected_hash
    {
        return Err(PersistenceError::ManifestJournalChecksumMismatch {
            canister_id: entry.canister_id.clone(),
            snapshot_id: entry.snapshot_id.clone(),
            manifest: manifest_hash.to_string(),
            journal: expected_hash.to_string(),
        });
    }

    Ok(())
}

// Verify the execution journal is bound to the exact persisted backup plan.
pub(super) fn verify_execution_integrity(
    plan: &BackupPlan,
    journal: &BackupExecutionJournal,
) -> Result<BackupExecutionIntegrityReport, PersistenceError> {
    if plan.plan_id != journal.plan_id {
        return Err(PersistenceError::PlanJournalMismatch {
            field: "plan_id",
            plan: plan.plan_id.clone(),
            journal: journal.plan_id.clone(),
        });
    }
    if plan.run_id != journal.run_id {
        return Err(PersistenceError::PlanJournalMismatch {
            field: "run_id",
            plan: plan.run_id.clone(),
            journal: journal.run_id.clone(),
        });
    }
    if plan.phases.len() != journal.operations.len() {
        return Err(PersistenceError::PlanJournalMismatch {
            field: "operation_count",
            plan: plan.phases.len().to_string(),
            journal: journal.operations.len().to_string(),
        });
    }

    for (phase, operation) in plan.phases.iter().zip(&journal.operations) {
        let expected_sequence = usize::try_from(phase.order).unwrap_or(usize::MAX);
        if expected_sequence != operation.sequence {
            return Err(PersistenceError::PlanJournalOperationMismatch {
                sequence: operation.sequence,
                field: "sequence",
                plan: expected_sequence.to_string(),
                journal: operation.sequence.to_string(),
            });
        }
        if phase.operation_id != operation.operation_id {
            return Err(PersistenceError::PlanJournalOperationMismatch {
                sequence: operation.sequence,
                field: "operation_id",
                plan: phase.operation_id.clone(),
                journal: operation.operation_id.clone(),
            });
        }
        if phase.kind != operation.kind {
            return Err(PersistenceError::PlanJournalOperationMismatch {
                sequence: operation.sequence,
                field: "kind",
                plan: format!("{:?}", phase.kind),
                journal: format!("{:?}", operation.kind),
            });
        }
        if phase.target_canister_id != operation.target_canister_id {
            return Err(PersistenceError::PlanJournalOperationMismatch {
                sequence: operation.sequence,
                field: "target_canister_id",
                plan: phase.target_canister_id.clone().unwrap_or_default(),
                journal: operation.target_canister_id.clone().unwrap_or_default(),
            });
        }
    }
    verify_terminal_mutation_receipts(journal)?;

    Ok(BackupExecutionIntegrityReport {
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        verified: true,
        plan_operations: plan.phases.len(),
        journal_operations: journal.operations.len(),
    })
}

fn verify_terminal_mutation_receipts(
    journal: &BackupExecutionJournal,
) -> Result<(), PersistenceError> {
    for operation in journal.operations.iter().filter(|operation| {
        operation_kind_requires_receipt(&operation.kind)
            && matches!(
                operation.state,
                BackupExecutionOperationState::Completed
                    | BackupExecutionOperationState::Failed
                    | BackupExecutionOperationState::Skipped
            )
    }) {
        let expected_outcome = receipt_outcome_for_state(&operation.state);
        let has_receipt = journal.operation_receipts.iter().any(|receipt| {
            receipt.sequence == operation.sequence
                && receipt.operation_id == operation.operation_id
                && receipt.kind == operation.kind
                && receipt.target_canister_id == operation.target_canister_id
                && receipt.outcome == expected_outcome
        });
        if !has_receipt {
            return Err(PersistenceError::ExecutionOperationMissingReceipt {
                sequence: operation.sequence,
                state: format!("{:?}", operation.state),
            });
        }
    }

    Ok(())
}

const fn operation_kind_requires_receipt(kind: &BackupOperationKind) -> bool {
    matches!(
        kind,
        BackupOperationKind::Stop
            | BackupOperationKind::CreateSnapshot
            | BackupOperationKind::Start
            | BackupOperationKind::DownloadSnapshot
            | BackupOperationKind::VerifyArtifact
            | BackupOperationKind::FinalizeManifest
    )
}

fn receipt_outcome_for_state(
    state: &BackupExecutionOperationState,
) -> BackupExecutionOperationReceiptOutcome {
    match state {
        BackupExecutionOperationState::Completed => {
            BackupExecutionOperationReceiptOutcome::Completed
        }
        BackupExecutionOperationState::Failed => BackupExecutionOperationReceiptOutcome::Failed,
        BackupExecutionOperationState::Skipped => BackupExecutionOperationReceiptOutcome::Skipped,
        BackupExecutionOperationState::Ready
        | BackupExecutionOperationState::Pending
        | BackupExecutionOperationState::Blocked => {
            unreachable!("non-terminal operation state does not have a receipt outcome")
        }
    }
}

// Compare manifest and journal topology receipts for fail-closed verification.
fn topology_receipt_mismatches(
    manifest: &FleetBackupManifest,
    journal: &DownloadJournal,
) -> Vec<TopologyReceiptMismatch> {
    let mut mismatches = Vec::new();
    record_topology_receipt_mismatch(
        &mut mismatches,
        "discovery_topology_hash",
        &manifest.fleet.discovery_topology_hash,
        journal.discovery_topology_hash.as_deref(),
    );
    record_topology_receipt_mismatch(
        &mut mismatches,
        "pre_snapshot_topology_hash",
        &manifest.fleet.pre_snapshot_topology_hash,
        journal.pre_snapshot_topology_hash.as_deref(),
    );
    mismatches
}

// Record one manifest/journal topology receipt mismatch.
fn record_topology_receipt_mismatch(
    mismatches: &mut Vec<TopologyReceiptMismatch>,
    field: &str,
    manifest: &str,
    journal: Option<&str>,
) {
    if journal == Some(manifest) {
        return;
    }

    mismatches.push(TopologyReceiptMismatch {
        field: field.to_string(),
        manifest: manifest.to_string(),
        journal: journal.map(ToString::to_string),
    });
}

/// Resolve a backup artifact path under the backup root.
#[must_use]
pub fn resolve_backup_artifact_path(root: &Path, artifact_path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(artifact_path);
    if path.is_absolute() {
        return None;
    }
    let is_safe = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if !is_safe {
        return None;
    }

    Some(root.join(path))
}
