use crate::{
    journal::DownloadJournal,
    manifest::{FleetBackupManifest, ManifestValidationError},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const MANIFEST_FILE_NAME: &str = "fleet-backup-manifest.json";
const JOURNAL_FILE_NAME: &str = "download-journal.json";

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

    /// Return the canonical mutable journal path for this backup layout.
    #[must_use]
    pub fn journal_path(&self) -> PathBuf {
        self.root.join(JOURNAL_FILE_NAME)
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
mod tests {
    use super::*;
    use crate::{
        journal::{ArtifactJournalEntry, ArtifactState},
        manifest::{
            BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetMember,
            FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
            VerificationCheck, VerificationPlan,
        },
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    const ROOT: &str = "aaaaa-aa";
    const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Ensure manifest writes create parent dirs and round-trip through validation.
    #[test]
    fn manifest_round_trips_through_layout() {
        let root = temp_dir("canic-backup-manifest-layout");
        let layout = BackupLayout::new(root.clone());
        let manifest = valid_manifest();

        layout
            .write_manifest(&manifest)
            .expect("write manifest atomically");
        let read = layout.read_manifest().expect("read manifest");

        fs::remove_dir_all(root).expect("remove temp layout");
        assert_eq!(read.backup_id, manifest.backup_id);
    }

    // Ensure journal writes create parent dirs and round-trip through validation.
    #[test]
    fn journal_round_trips_through_layout() {
        let root = temp_dir("canic-backup-journal-layout");
        let layout = BackupLayout::new(root.clone());
        let journal = valid_journal();

        layout
            .write_journal(&journal)
            .expect("write journal atomically");
        let read = layout.read_journal().expect("read journal");

        fs::remove_dir_all(root).expect("remove temp layout");
        assert_eq!(read.backup_id, journal.backup_id);
    }

    // Ensure invalid manifests are rejected before writing.
    #[test]
    fn invalid_manifest_is_not_written() {
        let root = temp_dir("canic-backup-invalid-manifest");
        let layout = BackupLayout::new(root.clone());
        let mut manifest = valid_manifest();
        manifest.fleet.discovery_topology_hash = "bad".to_string();

        let err = layout
            .write_manifest(&manifest)
            .expect_err("invalid manifest should fail");

        let manifest_path = layout.manifest_path();
        fs::remove_dir_all(root).ok();
        assert!(matches!(err, PersistenceError::InvalidManifest(_)));
        assert!(!manifest_path.exists());
    }

    // Build one valid manifest for persistence tests.
    fn valid_manifest() -> FleetBackupManifest {
        FleetBackupManifest {
            manifest_version: 1,
            backup_id: "fbk_test_001".to_string(),
            created_at: "2026-04-10T12:00:00Z".to_string(),
            tool: ToolMetadata {
                name: "canic".to_string(),
                version: "v1".to_string(),
            },
            source: SourceMetadata {
                environment: "local".to_string(),
                root_canister: ROOT.to_string(),
            },
            consistency: ConsistencySection {
                mode: ConsistencyMode::CrashConsistent,
                backup_units: vec![BackupUnit {
                    unit_id: "whole-fleet".to_string(),
                    kind: BackupUnitKind::WholeFleet,
                    roles: vec!["root".to_string()],
                    consistency_reason: None,
                    dependency_closure: Vec::new(),
                    topology_validation: "subtree-closed".to_string(),
                    quiescence_strategy: None,
                }],
            },
            fleet: FleetSection {
                topology_hash_algorithm: "sha256".to_string(),
                topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
                discovery_topology_hash: HASH.to_string(),
                pre_snapshot_topology_hash: HASH.to_string(),
                topology_hash: HASH.to_string(),
                members: vec![FleetMember {
                    role: "root".to_string(),
                    canister_id: ROOT.to_string(),
                    parent_canister_id: None,
                    subnet_canister_id: Some(CHILD.to_string()),
                    controller_hint: Some(ROOT.to_string()),
                    identity_mode: IdentityMode::Fixed,
                    restore_group: 1,
                    verification_class: "basic".to_string(),
                    verification_checks: vec![VerificationCheck {
                        kind: "call".to_string(),
                        method: Some("canic_ready".to_string()),
                        roles: Vec::new(),
                    }],
                    source_snapshot: SourceSnapshot {
                        snapshot_id: "snap-root".to_string(),
                        module_hash: Some(HASH.to_string()),
                        wasm_hash: Some(HASH.to_string()),
                        code_version: Some("v0.30.0".to_string()),
                        artifact_path: "artifacts/root".to_string(),
                        checksum_algorithm: "sha256".to_string(),
                    },
                }],
            },
            verification: VerificationPlan {
                fleet_checks: Vec::new(),
                member_checks: Vec::new(),
            },
        }
    }

    // Build one valid durable journal for persistence tests.
    fn valid_journal() -> DownloadJournal {
        DownloadJournal {
            journal_version: 1,
            backup_id: "fbk_test_001".to_string(),
            artifacts: vec![ArtifactJournalEntry {
                canister_id: ROOT.to_string(),
                snapshot_id: "snap-root".to_string(),
                state: ArtifactState::Durable,
                temp_path: None,
                artifact_path: "artifacts/root".to_string(),
                checksum_algorithm: "sha256".to_string(),
                checksum: Some(HASH.to_string()),
                updated_at: "2026-04-10T12:00:00Z".to_string(),
            }],
        }
    }

    // Build a unique temporary layout directory.
    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
