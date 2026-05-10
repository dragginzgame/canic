use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    execution::{BackupExecutionJournal, BackupExecutionJournalError},
    journal::{ArtifactState, DownloadJournal},
    manifest::{FleetBackupManifest, ManifestValidationError},
    plan::{BackupPlan, BackupPlanError},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io,
    path::{Component, Path, PathBuf},
};
use thiserror::Error as ThisError;

const MANIFEST_FILE_NAME: &str = "fleet-backup-manifest.json";
const BACKUP_PLAN_FILE_NAME: &str = "backup-plan.json";
const JOURNAL_FILE_NAME: &str = "download-journal.json";
const EXECUTION_JOURNAL_FILE_NAME: &str = "backup-execution-journal.json";

///
/// BackupLayout
///

#[derive(Clone, Debug)]
pub struct BackupLayout {
    root: PathBuf,
}

impl BackupLayout {
    /// Create a filesystem layout rooted at one backup directory.
    #[must_use]
    pub const fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Return the root backup directory path.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the canonical manifest path for this backup layout.
    #[must_use]
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE_NAME)
    }

    /// Return the canonical backup plan path for this layout.
    #[must_use]
    pub fn backup_plan_path(&self) -> PathBuf {
        self.root.join(BACKUP_PLAN_FILE_NAME)
    }

    /// Return the canonical mutable journal path for this backup layout.
    #[must_use]
    pub fn journal_path(&self) -> PathBuf {
        self.root.join(JOURNAL_FILE_NAME)
    }

    /// Return the canonical backup execution journal path for this layout.
    #[must_use]
    pub fn execution_journal_path(&self) -> PathBuf {
        self.root.join(EXECUTION_JOURNAL_FILE_NAME)
    }

    /// Write a validated manifest with atomic replace semantics.
    pub fn write_manifest(&self, manifest: &FleetBackupManifest) -> Result<(), PersistenceError> {
        manifest.validate()?;
        write_json_atomic(&self.manifest_path(), manifest)
    }

    /// Read and validate a manifest from this backup layout.
    pub fn read_manifest(&self) -> Result<FleetBackupManifest, PersistenceError> {
        let manifest = read_json(&self.manifest_path())?;
        FleetBackupManifest::validate(&manifest)?;
        Ok(manifest)
    }

    /// Write a validated backup plan with atomic replace semantics.
    pub fn write_backup_plan(&self, plan: &BackupPlan) -> Result<(), PersistenceError> {
        plan.validate()?;
        write_json_atomic(&self.backup_plan_path(), plan)
    }

    /// Read and validate a backup plan from this layout.
    pub fn read_backup_plan(&self) -> Result<BackupPlan, PersistenceError> {
        let plan = read_json(&self.backup_plan_path())?;
        BackupPlan::validate(&plan)?;
        Ok(plan)
    }

    /// Write a validated download journal with atomic replace semantics.
    pub fn write_journal(&self, journal: &DownloadJournal) -> Result<(), PersistenceError> {
        journal.validate()?;
        write_json_atomic(&self.journal_path(), journal)
    }

    /// Read and validate a download journal from this backup layout.
    pub fn read_journal(&self) -> Result<DownloadJournal, PersistenceError> {
        let journal = read_json(&self.journal_path())?;
        DownloadJournal::validate(&journal)?;
        Ok(journal)
    }

    /// Write a validated backup execution journal with atomic replace semantics.
    pub fn write_execution_journal(
        &self,
        journal: &BackupExecutionJournal,
    ) -> Result<(), PersistenceError> {
        journal.validate()?;
        write_json_atomic(&self.execution_journal_path(), journal)
    }

    /// Read and validate a backup execution journal from this layout.
    pub fn read_execution_journal(&self) -> Result<BackupExecutionJournal, PersistenceError> {
        let journal = read_json(&self.execution_journal_path())?;
        BackupExecutionJournal::validate(&journal)?;
        Ok(journal)
    }

    /// Validate the manifest, journal, and durable artifact checksums.
    pub fn verify_integrity(&self) -> Result<BackupIntegrityReport, PersistenceError> {
        let manifest = self.read_manifest()?;
        let journal = self.read_journal()?;
        verify_layout_integrity(self, &manifest, &journal)
    }

    /// Validate the persisted backup plan and execution journal agree.
    pub fn verify_execution_integrity(
        &self,
    ) -> Result<BackupExecutionIntegrityReport, PersistenceError> {
        let plan = self.read_backup_plan()?;
        let journal = self.read_execution_journal()?;
        verify_execution_integrity(&plan, &journal)
    }
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
/// PersistenceError
///

#[derive(Debug, ThisError)]
pub enum PersistenceError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),

    #[error(transparent)]
    InvalidJournal(#[from] crate::journal::JournalValidationError),

    #[error(transparent)]
    InvalidBackupPlan(#[from] BackupPlanError),

    #[error(transparent)]
    InvalidExecutionJournal(#[from] BackupExecutionJournalError),

    #[error(transparent)]
    Checksum(#[from] ArtifactChecksumError),

    #[error("manifest backup id {manifest} does not match journal backup id {journal}")]
    BackupIdMismatch { manifest: String, journal: String },

    #[error("journal artifact {canister_id} snapshot {snapshot_id} is not durable")]
    NonDurableArtifact {
        canister_id: String,
        snapshot_id: String,
    },

    #[error("manifest member {canister_id} snapshot {snapshot_id} has no journal artifact")]
    MissingJournalArtifact {
        canister_id: String,
        snapshot_id: String,
    },

    #[error("journal artifact {canister_id} snapshot {snapshot_id} is not declared in manifest")]
    UnexpectedJournalArtifact {
        canister_id: String,
        snapshot_id: String,
    },

    #[error(
        "manifest checksum for {canister_id} snapshot {snapshot_id} does not match journal checksum"
    )]
    ManifestJournalChecksumMismatch {
        canister_id: String,
        snapshot_id: String,
        manifest: String,
        journal: String,
    },

    #[error(
        "manifest artifact path for {canister_id} snapshot {snapshot_id} does not match journal artifact path"
    )]
    ManifestJournalArtifactPathMismatch {
        canister_id: String,
        snapshot_id: String,
        manifest: String,
        journal: String,
    },

    #[error("manifest topology receipt {field} does not match journal topology receipt")]
    ManifestJournalTopologyReceiptMismatch {
        field: String,
        manifest: String,
        journal: Option<String>,
    },

    #[error("backup plan {field} does not match execution journal")]
    PlanJournalMismatch {
        field: &'static str,
        plan: String,
        journal: String,
    },

    #[error("backup plan operation {sequence} {field} does not match execution journal")]
    PlanJournalOperationMismatch {
        sequence: usize,
        field: &'static str,
        plan: String,
        journal: String,
    },

    #[error("artifact path escapes backup root: {artifact_path}")]
    ArtifactPathEscapesBackup { artifact_path: String },

    #[error("artifact path does not exist: {0}")]
    MissingArtifact(String),
}

// Verify cross-file backup layout consistency and artifact checksums.
fn verify_layout_integrity(
    layout: &BackupLayout,
    manifest: &FleetBackupManifest,
    journal: &DownloadJournal,
) -> Result<BackupIntegrityReport, PersistenceError> {
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

    let expected_artifacts = manifest
        .fleet
        .members
        .iter()
        .map(|member| {
            (
                member.canister_id.as_str(),
                member.source_snapshot.snapshot_id.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
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

        let Some(expected_hash) = entry.checksum.as_deref() else {
            unreachable!("validated durable journals must include checksums");
        };
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
        artifacts.push(ArtifactIntegrityReport {
            canister_id: entry.canister_id.clone(),
            snapshot_id: entry.snapshot_id.clone(),
            artifact_path: artifact_path.display().to_string(),
            checksum: expected_hash.to_string(),
        });
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

// Verify the execution journal is bound to the exact persisted backup plan.
fn verify_execution_integrity(
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

    Ok(BackupExecutionIntegrityReport {
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        verified: true,
        plan_operations: plan.phases.len(),
        journal_operations: journal.operations.len(),
    })
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

// Write JSON to a temporary sibling path and then atomically replace the target.
fn write_json_atomic<T>(path: &Path, value: &T) -> Result<(), PersistenceError>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = temp_path_for(path);
    let mut file = File::create(&tmp_path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.sync_all()?;
    drop(file);

    fs::rename(&tmp_path, path)?;

    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }

    Ok(())
}

// Read one JSON document from disk.
fn read_json<T>(path: &Path) -> Result<T, PersistenceError>
where
    T: DeserializeOwned,
{
    let file = File::open(path)?;
    Ok(serde_json::from_reader(file)?)
}

// Build the sibling temporary path used for atomic writes.
fn temp_path_for(path: &Path) -> PathBuf {
    let mut file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("canic-backup")
        .to_string();
    file_name.push_str(".tmp");
    path.with_file_name(file_name)
}

#[cfg(test)]
mod tests;
