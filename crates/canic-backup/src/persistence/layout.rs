//! Module: persistence::layout
//!
//! Responsibility: define canonical backup layout paths and validated file IO.
//! Does not own: JSON mechanics, domain validation rules, or checksum verification.
//! Boundary: exposes filesystem persistence operations for backup workflows.

use crate::{
    execution::BackupExecutionJournal,
    journal::DownloadJournal,
    manifest::DeploymentBackupManifest,
    persistence::{
        BackupExecutionIntegrityReport, BackupIntegrityReport, PersistenceError,
        integrity::{verify_execution_integrity, verify_layout_integrity},
        json::{create_json_durable, read_json, write_json_durable},
    },
    plan::BackupPlan,
};

use std::path::{Path, PathBuf};

const MANIFEST_FILE_NAME: &str = "deployment-backup-manifest.json";
const BACKUP_PLAN_FILE_NAME: &str = "backup-plan.json";
const JOURNAL_FILE_NAME: &str = "download-journal.json";
const EXECUTION_JOURNAL_FILE_NAME: &str = "backup-execution-journal.json";

///
/// BackupLayout
///
/// Canonical filesystem layout for one backup directory.
/// Owned by persistence and used by backup runner and restore verification paths.
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

    /// Publish a validated manifest or adopt the exact existing manifest.
    pub fn publish_manifest(
        &self,
        manifest: &DeploymentBackupManifest,
    ) -> Result<(), PersistenceError> {
        manifest.validate()?;
        let path = self.manifest_path();
        if path.exists() {
            return self.require_exact_manifest(manifest);
        }
        match create_json_durable(&path, manifest) {
            Ok(()) => Ok(()),
            Err(PersistenceError::Io(error))
                if error.kind() == std::io::ErrorKind::AlreadyExists =>
            {
                self.require_exact_manifest(manifest)
            }
            Err(error) => Err(error),
        }
    }

    /// Read and validate a manifest from this backup layout.
    pub fn read_manifest(&self) -> Result<DeploymentBackupManifest, PersistenceError> {
        let manifest = read_json(&self.manifest_path())?;
        DeploymentBackupManifest::validate(&manifest)?;
        Ok(manifest)
    }

    fn require_exact_manifest(
        &self,
        expected: &DeploymentBackupManifest,
    ) -> Result<(), PersistenceError> {
        let path = self.manifest_path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(PersistenceError::ManifestConflict {
                path: path.display().to_string(),
            });
        }
        let actual = self.read_manifest()?;
        if serde_json::to_vec(&actual)? == serde_json::to_vec(expected)? {
            std::fs::File::open(&path)?.sync_all()?;
            if let Some(parent) = path.parent() {
                std::fs::File::open(parent)?.sync_all()?;
            }
            Ok(())
        } else {
            Err(PersistenceError::ManifestConflict {
                path: path.display().to_string(),
            })
        }
    }

    #[cfg(test)]
    pub(crate) fn publish_manifest_at_barriers(
        &self,
        manifest: &DeploymentBackupManifest,
        before_publication: impl FnMut(),
        after_directory_sync: impl FnMut(),
    ) -> Result<(), PersistenceError> {
        use crate::persistence::json::create_json_durable_at_barriers;

        manifest.validate()?;
        create_json_durable_at_barriers(
            &self.manifest_path(),
            manifest,
            before_publication,
            after_directory_sync,
        )
    }

    /// Write a validated backup plan with durable replace semantics.
    pub fn write_backup_plan(&self, plan: &BackupPlan) -> Result<(), PersistenceError> {
        plan.validate()?;
        write_json_durable(&self.backup_plan_path(), plan)
    }

    /// Read and validate a backup plan from this layout.
    pub fn read_backup_plan(&self) -> Result<BackupPlan, PersistenceError> {
        let plan = read_json(&self.backup_plan_path())?;
        BackupPlan::validate(&plan)?;
        Ok(plan)
    }

    /// Write a validated download journal with durable replace semantics.
    pub fn write_journal(&self, journal: &DownloadJournal) -> Result<(), PersistenceError> {
        journal.validate()?;
        write_json_durable(&self.journal_path(), journal)
    }

    /// Read and validate a download journal from this backup layout.
    pub fn read_journal(&self) -> Result<DownloadJournal, PersistenceError> {
        let journal = read_json(&self.journal_path())?;
        DownloadJournal::validate(&journal)?;
        Ok(journal)
    }

    /// Write a validated backup execution journal with durable replace semantics.
    pub fn write_execution_journal(
        &self,
        journal: &BackupExecutionJournal,
    ) -> Result<(), PersistenceError> {
        journal.validate()?;
        write_json_durable(&self.execution_journal_path(), journal)
    }

    #[cfg(all(test, unix))]
    pub(crate) fn write_execution_journal_at_barriers(
        &self,
        journal: &BackupExecutionJournal,
        barriers: impl FnMut(crate::persistence::DurableWriteBarrier),
    ) -> Result<(), PersistenceError> {
        use crate::persistence::write_json_durable_at_barriers;

        journal.validate()?;
        write_json_durable_at_barriers(&self.execution_journal_path(), journal, barriers)
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
