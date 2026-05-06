use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    discovery::{DiscoveryError, SnapshotTarget, parse_registry_entries, targets_from_registry},
    journal::{
        ArtifactJournalEntry, ArtifactState, DownloadJournal, DownloadOperationMetrics,
        JournalValidationError,
    },
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetBackupManifest,
        FleetMember, FleetSection, IdentityMode, ManifestValidationError, SourceMetadata,
        SourceSnapshot, ToolMetadata, VerificationCheck, VerificationPlan,
    },
    persistence::{BackupLayout, PersistenceError},
    topology::{TopologyHash, TopologyHasher, TopologyRecord},
};
use candid::Principal;
use std::{
    collections::BTreeSet,
    error::Error as StdError,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub type SnapshotDriverError = Box<dyn StdError + Send + Sync + 'static>;

///
/// SnapshotArtifact
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotArtifact {
    pub canister_id: String,
    pub snapshot_id: String,
    pub path: std::path::PathBuf,
    pub checksum: String,
}

///
/// SnapshotLifecycleMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotLifecycleMode {
    SnapshotOnly,
    StopBeforeSnapshot,
    ResumeAfterSnapshot,
    StopAndResume,
}

impl SnapshotLifecycleMode {
    /// Build the lifecycle mode from stop/resume flags.
    #[must_use]
    pub const fn from_flags(stop_before_snapshot: bool, resume_after_snapshot: bool) -> Self {
        match (stop_before_snapshot, resume_after_snapshot) {
            (false, false) => Self::SnapshotOnly,
            (true, false) => Self::StopBeforeSnapshot,
            (false, true) => Self::ResumeAfterSnapshot,
            (true, true) => Self::StopAndResume,
        }
    }

    /// Return whether snapshot capture should stop the canister first.
    #[must_use]
    pub const fn stop_before_snapshot(self) -> bool {
        matches!(self, Self::StopBeforeSnapshot | Self::StopAndResume)
    }

    /// Return whether snapshot capture should resume the canister afterward.
    #[must_use]
    pub const fn resume_after_snapshot(self) -> bool {
        matches!(self, Self::ResumeAfterSnapshot | Self::StopAndResume)
    }
}

///
/// SnapshotDownloadConfig
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDownloadConfig {
    pub canister: String,
    pub out: PathBuf,
    pub root: Option<String>,
    pub include_children: bool,
    pub recursive: bool,
    pub dry_run: bool,
    pub lifecycle: SnapshotLifecycleMode,
    pub backup_id: String,
    pub created_at: String,
    pub tool_name: String,
    pub tool_version: String,
    pub environment: String,
}

///
/// SnapshotDownloadResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDownloadResult {
    pub artifacts: Vec<SnapshotArtifact>,
    pub planned_commands: Vec<String>,
}

///
/// SnapshotDownloadError
///

#[derive(Debug, ThisError)]
pub enum SnapshotDownloadError {
    #[error("missing --root when using --include-children")]
    MissingRegistrySource,

    #[error("snapshot driver failed: {0}")]
    Driver(#[source] SnapshotDriverError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Checksum(#[from] ArtifactChecksumError),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    Journal(#[from] JournalValidationError),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error(transparent)]
    Manifest(#[from] SnapshotManifestError),
}

///
/// SnapshotDriver
///

pub trait SnapshotDriver {
    /// Load the root registry JSON used to resolve child snapshot targets.
    fn registry_json(&mut self, root: &str) -> Result<String, SnapshotDriverError>;

    /// Create one canister snapshot and return its snapshot id.
    fn create_snapshot(&mut self, canister_id: &str) -> Result<String, SnapshotDriverError>;

    /// Stop one canister before snapshot creation.
    fn stop_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError>;

    /// Start one canister after snapshot capture.
    fn start_canister(&mut self, canister_id: &str) -> Result<(), SnapshotDriverError>;

    /// Download one snapshot into the supplied artifact directory.
    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), SnapshotDriverError>;

    /// Render the planned create command for dry-run output.
    fn create_snapshot_command(&self, canister_id: &str) -> String;

    /// Render the planned stop command for dry-run output.
    fn stop_canister_command(&self, canister_id: &str) -> String;

    /// Render the planned start command for dry-run output.
    fn start_canister_command(&self, canister_id: &str) -> String;

    /// Render the planned download command for dry-run output.
    fn download_snapshot_command(
        &self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> String;
}

///
/// SnapshotArtifactPaths
///

struct SnapshotArtifactPaths {
    relative_path: PathBuf,
    artifact_path: PathBuf,
    temp_path: PathBuf,
}

impl SnapshotArtifactPaths {
    // Build the durable and temporary filesystem paths for one snapshot target.
    fn new(root: &Path, canister_id: &str) -> Self {
        let relative_path = PathBuf::from(safe_path_segment(canister_id));
        let artifact_path = root.join(&relative_path);
        let temp_path = root.join(format!("{}.tmp", safe_path_segment(canister_id)));

        Self {
            relative_path,
            artifact_path,
            temp_path,
        }
    }
}

///
/// SnapshotManifestInput
///

pub struct SnapshotManifestInput<'a> {
    pub backup_id: String,
    pub created_at: String,
    pub tool_name: String,
    pub tool_version: String,
    pub environment: String,
    pub root_canister: String,
    pub selected_canister: String,
    pub include_children: bool,
    pub targets: &'a [SnapshotTarget],
    pub artifacts: &'a [SnapshotArtifact],
    pub discovery_topology_hash: TopologyHash,
    pub pre_snapshot_topology_hash: TopologyHash,
}

///
/// SnapshotManifestError
///

#[derive(Debug, ThisError)]
pub enum SnapshotManifestError {
    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error(
        "topology changed before snapshot start: discovery={discovery}, pre_snapshot={pre_snapshot}"
    )]
    TopologyChanged {
        discovery: String,
        pre_snapshot: String,
    },

    #[error("missing snapshot artifact for canister {0}")]
    MissingArtifact(String),

    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),
}

/// Create and download snapshots for the selected canister set.
pub fn download_snapshots(
    config: &SnapshotDownloadConfig,
    driver: &mut impl SnapshotDriver,
) -> Result<SnapshotDownloadResult, SnapshotDownloadError> {
    let targets = resolve_snapshot_targets(config, driver)?;
    let discovery_topology_hash = topology_hash_for_targets(&config.canister, &targets)?;
    let pre_snapshot_topology_hash =
        accepted_pre_snapshot_topology_hash(config, driver, &discovery_topology_hash)?;
    let layout = BackupLayout::new(config.out.clone());
    let mut artifacts = Vec::with_capacity(targets.len());
    let mut planned_commands = Vec::new();
    let mut journal = DownloadJournal {
        journal_version: 1,
        backup_id: config.backup_id.clone(),
        discovery_topology_hash: Some(discovery_topology_hash.hash.clone()),
        pre_snapshot_topology_hash: Some(pre_snapshot_topology_hash.hash.clone()),
        operation_metrics: DownloadOperationMetrics {
            target_count: targets.len(),
            ..DownloadOperationMetrics::default()
        },
        artifacts: Vec::new(),
    };

    for target in &targets {
        let paths = SnapshotArtifactPaths::new(&config.out, &target.canister_id);

        if config.dry_run {
            let (artifact, commands) =
                dry_run_artifact(config, driver, target, paths.artifact_path);
            artifacts.push(artifact);
            planned_commands.extend(commands);
            continue;
        }

        artifacts.push(capture_snapshot_artifact(
            config,
            driver,
            &layout,
            &mut journal,
            target,
            paths,
        )?);
    }

    if !config.dry_run {
        let manifest = build_snapshot_manifest(SnapshotManifestInput {
            backup_id: config.backup_id.clone(),
            created_at: config.created_at.clone(),
            tool_name: config.tool_name.clone(),
            tool_version: config.tool_version.clone(),
            environment: config.environment.clone(),
            root_canister: config
                .root
                .clone()
                .unwrap_or_else(|| config.canister.clone()),
            selected_canister: config.canister.clone(),
            include_children: config.include_children,
            targets: &targets,
            artifacts: &artifacts,
            discovery_topology_hash,
            pre_snapshot_topology_hash,
        })?;
        layout.write_manifest(&manifest)?;
    }

    Ok(SnapshotDownloadResult {
        artifacts,
        planned_commands,
    })
}

/// Resolve the selected canister plus optional direct/recursive children.
pub fn resolve_snapshot_targets(
    config: &SnapshotDownloadConfig,
    driver: &mut impl SnapshotDriver,
) -> Result<Vec<SnapshotTarget>, SnapshotDownloadError> {
    if !config.include_children {
        return Ok(vec![SnapshotTarget {
            canister_id: config.canister.clone(),
            role: None,
            parent_canister_id: None,
        }]);
    }

    let registry_json = if let Some(root) = &config.root {
        driver
            .registry_json(root)
            .map_err(SnapshotDownloadError::Driver)?
    } else {
        return Err(SnapshotDownloadError::MissingRegistrySource);
    };
    let registry = parse_registry_entries(&registry_json)?;
    targets_from_registry(&registry, &config.canister, config.recursive)
        .map_err(SnapshotDownloadError::from)
}

/// Build a validated fleet backup manifest for one successful snapshot run.
pub fn build_snapshot_manifest(
    input: SnapshotManifestInput<'_>,
) -> Result<FleetBackupManifest, SnapshotManifestError> {
    let roles = input
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| target_role(&input.selected_canister, index, target))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let manifest = FleetBackupManifest {
        manifest_version: 1,
        backup_id: input.backup_id,
        created_at: input.created_at,
        tool: ToolMetadata {
            name: input.tool_name,
            version: input.tool_version,
        },
        source: SourceMetadata {
            environment: input.environment,
            root_canister: input.root_canister.clone(),
        },
        consistency: ConsistencySection {
            mode: ConsistencyMode::CrashConsistent,
            backup_units: vec![BackupUnit {
                unit_id: "snapshot-selection".to_string(),
                kind: if input.include_children {
                    BackupUnitKind::SubtreeRooted
                } else {
                    BackupUnitKind::Flat
                },
                roles,
                consistency_reason: if input.include_children {
                    None
                } else {
                    Some("explicit single-canister snapshot selection".to_string())
                },
                dependency_closure: Vec::new(),
                topology_validation: if input.include_children {
                    "registry-subtree-selection".to_string()
                } else {
                    "explicit-selection".to_string()
                },
                quiescence_strategy: None,
            }],
        },
        fleet: FleetSection {
            topology_hash_algorithm: input.discovery_topology_hash.algorithm,
            topology_hash_input: input.discovery_topology_hash.input,
            discovery_topology_hash: input.discovery_topology_hash.hash.clone(),
            pre_snapshot_topology_hash: input.pre_snapshot_topology_hash.hash,
            topology_hash: input.discovery_topology_hash.hash,
            members: input
                .targets
                .iter()
                .enumerate()
                .map(|(index, target)| {
                    fleet_member(
                        &input.selected_canister,
                        Some(input.root_canister.as_str()).filter(|_| input.include_children),
                        index,
                        target,
                        input.artifacts,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        },
        verification: VerificationPlan::default(),
    };

    manifest.validate()?;
    Ok(manifest)
}

/// Compute the canonical topology hash for one resolved target set.
pub fn topology_hash_for_targets(
    selected_canister: &str,
    targets: &[SnapshotTarget],
) -> Result<TopologyHash, SnapshotManifestError> {
    let topology_records = targets
        .iter()
        .enumerate()
        .map(|(index, target)| topology_record(selected_canister, index, target))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TopologyHasher::hash(&topology_records))
}

/// Fail closed if topology changes after discovery but before snapshot creation.
pub fn ensure_topology_stable(
    discovery: &TopologyHash,
    pre_snapshot: &TopologyHash,
) -> Result<(), SnapshotManifestError> {
    if discovery.hash == pre_snapshot.hash {
        return Ok(());
    }

    Err(SnapshotManifestError::TopologyChanged {
        discovery: discovery.hash.clone(),
        pre_snapshot: pre_snapshot.hash.clone(),
    })
}

// Resolve and verify the pre-snapshot topology hash before any mutation.
fn accepted_pre_snapshot_topology_hash(
    config: &SnapshotDownloadConfig,
    driver: &mut impl SnapshotDriver,
    discovery_topology_hash: &TopologyHash,
) -> Result<TopologyHash, SnapshotDownloadError> {
    if config.dry_run {
        return Ok(discovery_topology_hash.clone());
    }

    let pre_snapshot_targets = resolve_snapshot_targets(config, driver)?;
    let pre_snapshot_topology_hash =
        topology_hash_for_targets(&config.canister, &pre_snapshot_targets)?;
    ensure_topology_stable(discovery_topology_hash, &pre_snapshot_topology_hash)?;
    Ok(pre_snapshot_topology_hash)
}

// Return dry-run commands and a placeholder artifact without mutating state.
fn dry_run_artifact(
    config: &SnapshotDownloadConfig,
    driver: &impl SnapshotDriver,
    target: &SnapshotTarget,
    artifact_path: PathBuf,
) -> (SnapshotArtifact, Vec<String>) {
    let mut commands = Vec::new();
    if config.lifecycle.stop_before_snapshot() {
        commands.push(driver.stop_canister_command(&target.canister_id));
    }
    commands.push(driver.create_snapshot_command(&target.canister_id));
    commands.push(driver.download_snapshot_command(
        &target.canister_id,
        "<snapshot-id>",
        &artifact_path,
    ));
    if config.lifecycle.resume_after_snapshot() {
        commands.push(driver.start_canister_command(&target.canister_id));
    }

    (
        SnapshotArtifact {
            canister_id: target.canister_id.clone(),
            snapshot_id: "<snapshot-id>".to_string(),
            path: artifact_path,
            checksum: "<sha256>".to_string(),
        },
        commands,
    )
}

// Create, download, checksum, and finalize one durable snapshot artifact.
fn capture_snapshot_artifact(
    config: &SnapshotDownloadConfig,
    driver: &mut impl SnapshotDriver,
    layout: &BackupLayout,
    journal: &mut DownloadJournal,
    target: &SnapshotTarget,
    paths: SnapshotArtifactPaths,
) -> Result<SnapshotArtifact, SnapshotDownloadError> {
    if config.lifecycle.stop_before_snapshot() {
        driver
            .stop_canister(&target.canister_id)
            .map_err(SnapshotDownloadError::Driver)?;
    }

    let result = capture_snapshot_artifact_body(
        driver,
        layout,
        journal,
        target,
        &paths.relative_path,
        paths.artifact_path,
        paths.temp_path,
    );

    if config.lifecycle.resume_after_snapshot() {
        match result {
            Ok(artifact) => {
                driver
                    .start_canister(&target.canister_id)
                    .map_err(SnapshotDownloadError::Driver)?;
                Ok(artifact)
            }
            Err(error) => {
                let _ = driver.start_canister(&target.canister_id);
                Err(error)
            }
        }
    } else {
        result
    }
}

// Run the mutation-heavy capture path after lifecycle handling is settled.
fn capture_snapshot_artifact_body(
    driver: &mut impl SnapshotDriver,
    layout: &BackupLayout,
    journal: &mut DownloadJournal,
    target: &SnapshotTarget,
    artifact_relative_path: &Path,
    artifact_path: PathBuf,
    temp_path: PathBuf,
) -> Result<SnapshotArtifact, SnapshotDownloadError> {
    journal.operation_metrics.snapshot_create_started += 1;
    let snapshot_id = driver
        .create_snapshot(&target.canister_id)
        .map_err(SnapshotDownloadError::Driver)?;
    journal.operation_metrics.snapshot_create_completed += 1;
    let mut entry = ArtifactJournalEntry {
        canister_id: target.canister_id.clone(),
        snapshot_id: snapshot_id.clone(),
        state: ArtifactState::Created,
        temp_path: None,
        artifact_path: artifact_relative_path.display().to_string(),
        checksum_algorithm: "sha256".to_string(),
        checksum: None,
        updated_at: "unknown".to_string(),
    };
    journal.artifacts.push(entry.clone());
    layout.write_journal(journal)?;

    if temp_path.exists() {
        fs::remove_dir_all(&temp_path)?;
    }
    fs::create_dir_all(&temp_path)?;
    journal.operation_metrics.snapshot_download_started += 1;
    layout.write_journal(journal)?;
    driver
        .download_snapshot(&target.canister_id, &snapshot_id, &temp_path)
        .map_err(SnapshotDownloadError::Driver)?;
    journal.operation_metrics.snapshot_download_completed += 1;
    entry.advance_to(ArtifactState::Downloaded, "unknown".to_string())?;
    entry.temp_path = Some(temp_path.display().to_string());
    update_journal_entry(journal, &entry);
    layout.write_journal(journal)?;

    journal.operation_metrics.checksum_verify_started += 1;
    layout.write_journal(journal)?;
    let checksum = ArtifactChecksum::from_path(&temp_path)?;
    journal.operation_metrics.checksum_verify_completed += 1;
    entry.checksum = Some(checksum.hash.clone());
    entry.advance_to(ArtifactState::ChecksumVerified, "unknown".to_string())?;
    update_journal_entry(journal, &entry);
    layout.write_journal(journal)?;

    journal.operation_metrics.artifact_finalize_started += 1;
    layout.write_journal(journal)?;
    if artifact_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("artifact path already exists: {}", artifact_path.display()),
        )
        .into());
    }
    fs::rename(&temp_path, &artifact_path)?;
    journal.operation_metrics.artifact_finalize_completed += 1;
    entry.temp_path = None;
    entry.advance_to(ArtifactState::Durable, "unknown".to_string())?;
    update_journal_entry(journal, &entry);
    layout.write_journal(journal)?;

    Ok(SnapshotArtifact {
        canister_id: target.canister_id.clone(),
        snapshot_id,
        path: artifact_path,
        checksum: checksum.hash,
    })
}

// Replace one artifact row in the mutable journal.
fn update_journal_entry(journal: &mut DownloadJournal, entry: &ArtifactJournalEntry) {
    if let Some(existing) = journal.artifacts.iter_mut().find(|existing| {
        existing.canister_id == entry.canister_id && existing.snapshot_id == entry.snapshot_id
    }) {
        *existing = entry.clone();
    }
}

// Build one manifest member from a captured durable artifact.
fn fleet_member(
    selected_canister: &str,
    subnet_canister_id: Option<&str>,
    index: usize,
    target: &SnapshotTarget,
    artifacts: &[SnapshotArtifact],
) -> Result<FleetMember, SnapshotManifestError> {
    let Some(artifact) = artifacts
        .iter()
        .find(|artifact| artifact.canister_id == target.canister_id)
    else {
        return Err(SnapshotManifestError::MissingArtifact(
            target.canister_id.clone(),
        ));
    };
    let role = target_role(selected_canister, index, target);

    Ok(FleetMember {
        role: role.clone(),
        canister_id: target.canister_id.clone(),
        parent_canister_id: target.parent_canister_id.clone(),
        subnet_canister_id: subnet_canister_id.map(str::to_string),
        controller_hint: None,
        identity_mode: if target.canister_id == selected_canister {
            IdentityMode::Fixed
        } else {
            IdentityMode::Relocatable
        },
        restore_group: if target.canister_id == selected_canister {
            1
        } else {
            2
        },
        verification_class: "basic".to_string(),
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            method: None,
            roles: vec![role],
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: artifact.snapshot_id.clone(),
            module_hash: None,
            wasm_hash: None,
            code_version: None,
            artifact_path: safe_path_segment(&target.canister_id),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(artifact.checksum.clone()),
        },
    })
}

// Build one canonical topology record for manifest hashing.
fn topology_record(
    selected_canister: &str,
    index: usize,
    target: &SnapshotTarget,
) -> Result<TopologyRecord, SnapshotManifestError> {
    Ok(TopologyRecord {
        pid: parse_principal("fleet.members[].canister_id", &target.canister_id)?,
        parent_pid: target
            .parent_canister_id
            .as_deref()
            .map(|parent| parse_principal("fleet.members[].parent_canister_id", parent))
            .transpose()?,
        role: target_role(selected_canister, index, target),
        module_hash: None,
    })
}

// Return the manifest role for one selected snapshot target.
fn target_role(selected_canister: &str, index: usize, target: &SnapshotTarget) -> String {
    target.role.clone().unwrap_or_else(|| {
        if target.canister_id == selected_canister {
            "root".to_string()
        } else {
            format!("member-{index}")
        }
    })
}

// Parse one principal used by generated topology manifest metadata.
fn parse_principal(field: &'static str, value: &str) -> Result<Principal, SnapshotManifestError> {
    Principal::from_text(value).map_err(|_| SnapshotManifestError::InvalidPrincipal {
        field,
        value: value.to_string(),
    })
}

// Sanitize a canister id into a relative artifact directory segment.
fn safe_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests;
