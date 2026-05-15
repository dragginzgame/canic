mod integrity;

pub use integrity::{
    ArtifactIntegrityReport, BackupExecutionIntegrityReport, BackupIntegrityReport,
    resolve_backup_artifact_path,
};
use integrity::{verify_execution_integrity, verify_layout_integrity};

#[cfg(test)]
use crate::artifacts::ArtifactChecksum;
use crate::{
    artifacts::ArtifactChecksumError,
    execution::{BackupExecutionJournal, BackupExecutionJournalError},
    journal::DownloadJournal,
    manifest::{FleetBackupManifest, ManifestValidationError},
    plan::{BackupPlan, BackupPlanError},
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
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

    #[error("journal artifact {canister_id} snapshot {snapshot_id} has no checksum")]
    MissingJournalArtifactChecksum {
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

    #[error("backup execution operation {sequence} is {state} but has no matching receipt")]
    ExecutionOperationMissingReceipt { sequence: usize, state: String },

    #[error("backup execution operation {sequence} timestamp does not match latest receipt")]
    ExecutionOperationReceiptTimestampMismatch { sequence: usize },

    #[error("artifact path escapes backup root: {artifact_path}")]
    ArtifactPathEscapesBackup { artifact_path: String },

    #[error("artifact path does not exist: {0}")]
    MissingArtifact(String),
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
