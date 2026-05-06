use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    journal::{ArtifactState, DownloadJournal},
    manifest::{
        FleetBackupManifest, ManifestValidationError, backup_unit_kind_name, consistency_mode_name,
    },
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
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

    /// Validate the manifest, journal, and durable artifact checksums.
    pub fn verify_integrity(&self) -> Result<BackupIntegrityReport, PersistenceError> {
        let manifest = self.read_manifest()?;
        let journal = self.read_journal()?;
        verify_layout_integrity(self, &manifest, &journal)
    }

    /// Inspect manifest and journal agreement without reading artifact bytes.
    pub fn inspect(&self) -> Result<BackupInspectionReport, PersistenceError> {
        let manifest = self.read_manifest()?;
        let journal = self.read_journal()?;
        Ok(inspect_layout(&manifest, &journal))
    }

    /// Build an audit-oriented provenance report without reading artifact bytes.
    pub fn provenance(&self) -> Result<BackupProvenanceReport, PersistenceError> {
        let manifest = self.read_manifest()?;
        let journal = self.read_journal()?;
        Ok(provenance_report(&manifest, &journal))
    }
}

///
/// BackupProvenanceReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupProvenanceReport {
    pub backup_id: String,
    pub manifest_backup_id: String,
    pub journal_backup_id: String,
    pub backup_id_matches: bool,
    pub manifest_version: u16,
    pub journal_version: u16,
    pub created_at: String,
    pub tool_name: String,
    pub tool_version: String,
    pub source_environment: String,
    pub source_root_canister: String,
    pub topology_hash_algorithm: String,
    pub topology_hash_input: String,
    pub discovery_topology_hash: String,
    pub pre_snapshot_topology_hash: String,
    pub accepted_topology_hash: String,
    pub journal_discovery_topology_hash: Option<String>,
    pub journal_pre_snapshot_topology_hash: Option<String>,
    pub topology_receipts_match: bool,
    pub topology_receipt_mismatches: Vec<TopologyReceiptMismatch>,
    pub backup_unit_count: usize,
    pub member_count: usize,
    pub consistency_mode: String,
    pub backup_units: Vec<BackupUnitProvenance>,
    pub members: Vec<MemberSnapshotProvenance>,
}

///
/// BackupUnitProvenance
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupUnitProvenance {
    pub unit_id: String,
    pub kind: String,
    pub roles: Vec<String>,
    pub consistency_reason: Option<String>,
    pub dependency_closure: Vec<String>,
    pub topology_validation: String,
    pub quiescence_strategy: Option<String>,
}

///
/// MemberSnapshotProvenance
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemberSnapshotProvenance {
    pub canister_id: String,
    pub role: String,
    pub parent_canister_id: Option<String>,
    pub subnet_canister_id: Option<String>,
    pub identity_mode: String,
    pub restore_group: u16,
    pub verification_class: String,
    pub verification_checks: usize,
    pub snapshot_id: String,
    pub module_hash: Option<String>,
    pub wasm_hash: Option<String>,
    pub code_version: Option<String>,
    pub artifact_path: String,
    pub checksum_algorithm: String,
    pub manifest_checksum: Option<String>,
    pub journal_state: Option<String>,
    pub journal_checksum: Option<String>,
    pub journal_updated_at: Option<String>,
}

///
/// BackupInspectionReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupInspectionReport {
    pub backup_id: String,
    pub manifest_backup_id: String,
    pub journal_backup_id: String,
    pub backup_id_matches: bool,
    pub journal_complete: bool,
    pub ready_for_verify: bool,
    pub manifest_members: usize,
    pub journal_artifacts: usize,
    pub matched_artifacts: usize,
    pub topology_receipt_mismatches: Vec<TopologyReceiptMismatch>,
    pub missing_journal_artifacts: Vec<ArtifactReference>,
    pub unexpected_journal_artifacts: Vec<ArtifactReference>,
    pub path_mismatches: Vec<ArtifactPathMismatch>,
    pub checksum_mismatches: Vec<ArtifactChecksumMismatch>,
}

///
/// TopologyReceiptMismatch
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TopologyReceiptMismatch {
    pub field: String,
    pub manifest: String,
    pub journal: Option<String>,
}

///
/// ArtifactReference
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ArtifactReference {
    pub canister_id: String,
    pub snapshot_id: String,
}

///
/// ArtifactPathMismatch
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactPathMismatch {
    pub canister_id: String,
    pub snapshot_id: String,
    pub manifest: String,
    pub journal: String,
}

///
/// ArtifactChecksumMismatch
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactChecksumMismatch {
    pub canister_id: String,
    pub snapshot_id: String,
    pub manifest: String,
    pub journal: String,
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

    #[error("artifact path does not exist: {0}")]
    MissingArtifact(String),
}

// Inspect manifest and journal agreement without touching artifact contents.
fn inspect_layout(
    manifest: &FleetBackupManifest,
    journal: &DownloadJournal,
) -> BackupInspectionReport {
    let journal_report = journal.resume_report();
    let journal_artifacts = journal
        .artifacts
        .iter()
        .map(|entry| (artifact_key(&entry.canister_id, &entry.snapshot_id), entry))
        .collect::<BTreeMap<_, _>>();
    let manifest_artifacts = manifest
        .fleet
        .members
        .iter()
        .map(|member| {
            (
                artifact_key(&member.canister_id, &member.source_snapshot.snapshot_id),
                member,
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut matched_artifacts = 0;
    let mut missing_journal_artifacts = Vec::new();
    let mut path_mismatches = Vec::new();
    let mut checksum_mismatches = Vec::new();

    for (key, member) in &manifest_artifacts {
        let Some(entry) = journal_artifacts.get(key) else {
            missing_journal_artifacts.push(artifact_reference(key));
            continue;
        };

        matched_artifacts += 1;
        if member.source_snapshot.artifact_path != entry.artifact_path {
            path_mismatches.push(ArtifactPathMismatch {
                canister_id: key.0.clone(),
                snapshot_id: key.1.clone(),
                manifest: member.source_snapshot.artifact_path.clone(),
                journal: entry.artifact_path.clone(),
            });
        }

        if let (Some(manifest_hash), Some(journal_hash)) = (
            member.source_snapshot.checksum.as_deref(),
            entry.checksum.as_deref(),
        ) && manifest_hash != journal_hash
        {
            checksum_mismatches.push(ArtifactChecksumMismatch {
                canister_id: key.0.clone(),
                snapshot_id: key.1.clone(),
                manifest: manifest_hash.to_string(),
                journal: journal_hash.to_string(),
            });
        }
    }

    let unexpected_journal_artifacts = journal_artifacts
        .keys()
        .filter(|key| !manifest_artifacts.contains_key(*key))
        .map(artifact_reference)
        .collect::<Vec<_>>();
    let topology_receipt_mismatches = topology_receipt_mismatches(manifest, journal);
    let topology_receipts_match = topology_receipt_mismatches.is_empty();
    let backup_id_matches = manifest.backup_id == journal.backup_id;
    let ready_for_verify = backup_id_matches
        && topology_receipts_match
        && journal_report.is_complete
        && missing_journal_artifacts.is_empty()
        && unexpected_journal_artifacts.is_empty()
        && path_mismatches.is_empty()
        && checksum_mismatches.is_empty();

    BackupInspectionReport {
        backup_id: manifest.backup_id.clone(),
        manifest_backup_id: manifest.backup_id.clone(),
        journal_backup_id: journal.backup_id.clone(),
        backup_id_matches,
        journal_complete: journal_report.is_complete,
        ready_for_verify,
        manifest_members: manifest.fleet.members.len(),
        journal_artifacts: journal.artifacts.len(),
        matched_artifacts,
        topology_receipt_mismatches,
        missing_journal_artifacts,
        unexpected_journal_artifacts,
        path_mismatches,
        checksum_mismatches,
    }
}

// Build an audit-friendly manifest and journal provenance projection.
fn provenance_report(
    manifest: &FleetBackupManifest,
    journal: &DownloadJournal,
) -> BackupProvenanceReport {
    let journal_artifacts = journal
        .artifacts
        .iter()
        .map(|entry| (artifact_key(&entry.canister_id, &entry.snapshot_id), entry))
        .collect::<BTreeMap<_, _>>();
    let topology_receipt_mismatches = topology_receipt_mismatches(manifest, journal);
    let topology_receipts_match = topology_receipt_mismatches.is_empty();

    BackupProvenanceReport {
        backup_id: manifest.backup_id.clone(),
        manifest_backup_id: manifest.backup_id.clone(),
        journal_backup_id: journal.backup_id.clone(),
        backup_id_matches: manifest.backup_id == journal.backup_id,
        manifest_version: manifest.manifest_version,
        journal_version: journal.journal_version,
        created_at: manifest.created_at.clone(),
        tool_name: manifest.tool.name.clone(),
        tool_version: manifest.tool.version.clone(),
        source_environment: manifest.source.environment.clone(),
        source_root_canister: manifest.source.root_canister.clone(),
        topology_hash_algorithm: manifest.fleet.topology_hash_algorithm.clone(),
        topology_hash_input: manifest.fleet.topology_hash_input.clone(),
        discovery_topology_hash: manifest.fleet.discovery_topology_hash.clone(),
        pre_snapshot_topology_hash: manifest.fleet.pre_snapshot_topology_hash.clone(),
        accepted_topology_hash: manifest.fleet.topology_hash.clone(),
        journal_discovery_topology_hash: journal.discovery_topology_hash.clone(),
        journal_pre_snapshot_topology_hash: journal.pre_snapshot_topology_hash.clone(),
        topology_receipts_match,
        topology_receipt_mismatches,
        backup_unit_count: manifest.consistency.backup_units.len(),
        member_count: manifest.fleet.members.len(),
        consistency_mode: consistency_mode_name(&manifest.consistency.mode).to_string(),
        backup_units: manifest
            .consistency
            .backup_units
            .iter()
            .map(|unit| BackupUnitProvenance {
                unit_id: unit.unit_id.clone(),
                kind: backup_unit_kind_name(&unit.kind).to_string(),
                roles: unit.roles.clone(),
                consistency_reason: unit.consistency_reason.clone(),
                dependency_closure: unit.dependency_closure.clone(),
                topology_validation: unit.topology_validation.clone(),
                quiescence_strategy: unit.quiescence_strategy.clone(),
            })
            .collect(),
        members: manifest
            .fleet
            .members
            .iter()
            .map(|member| {
                let journal_entry = journal_artifacts.get(&artifact_key(
                    &member.canister_id,
                    &member.source_snapshot.snapshot_id,
                ));

                MemberSnapshotProvenance {
                    canister_id: member.canister_id.clone(),
                    role: member.role.clone(),
                    parent_canister_id: member.parent_canister_id.clone(),
                    subnet_canister_id: member.subnet_canister_id.clone(),
                    identity_mode: identity_mode_name(&member.identity_mode).to_string(),
                    restore_group: member.restore_group,
                    verification_class: member.verification_class.clone(),
                    verification_checks: member.verification_checks.len(),
                    snapshot_id: member.source_snapshot.snapshot_id.clone(),
                    module_hash: member.source_snapshot.module_hash.clone(),
                    wasm_hash: member.source_snapshot.wasm_hash.clone(),
                    code_version: member.source_snapshot.code_version.clone(),
                    artifact_path: member.source_snapshot.artifact_path.clone(),
                    checksum_algorithm: member.source_snapshot.checksum_algorithm.clone(),
                    manifest_checksum: member.source_snapshot.checksum.clone(),
                    journal_state: journal_entry
                        .map(|entry| artifact_state_name(entry.state).to_string()),
                    journal_checksum: journal_entry.and_then(|entry| entry.checksum.clone()),
                    journal_updated_at: journal_entry.map(|entry| entry.updated_at.clone()),
                }
            })
            .collect(),
    }
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
        let artifact_path = resolve_artifact_path(layout.root(), &entry.artifact_path);
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

// Build the stable key used to compare manifest members with journal artifacts.
fn artifact_key(canister_id: &str, snapshot_id: &str) -> (String, String) {
    (canister_id.to_string(), snapshot_id.to_string())
}

// Convert one artifact key into the public report shape.
fn artifact_reference(key: &(String, String)) -> ArtifactReference {
    ArtifactReference {
        canister_id: key.0.clone(),
        snapshot_id: key.1.clone(),
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

// Return the stable serialized name for an identity mode.
const fn identity_mode_name(mode: &crate::manifest::IdentityMode) -> &'static str {
    match mode {
        crate::manifest::IdentityMode::Fixed => "fixed",
        crate::manifest::IdentityMode::Relocatable => "relocatable",
    }
}

// Return the stable serialized name for a journal artifact state.
const fn artifact_state_name(state: ArtifactState) -> &'static str {
    match state {
        ArtifactState::Created => "Created",
        ArtifactState::Downloaded => "Downloaded",
        ArtifactState::ChecksumVerified => "ChecksumVerified",
        ArtifactState::Durable => "Durable",
    }
}

// Resolve artifact paths from either absolute, cwd-relative, or layout-relative values.
fn resolve_artifact_path(root: &Path, artifact_path: &str) -> PathBuf {
    let path = PathBuf::from(artifact_path);
    if path.is_absolute() || path.exists() {
        path
    } else {
        root.join(path)
    }
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
