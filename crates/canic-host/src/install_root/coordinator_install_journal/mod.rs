//! Module: install_root::coordinator_install_journal
//!
//! Responsibility: own canonical host recovery authority for the initial Fleet Coordinator.
//! Does not own: network commands, artifact reads, or Fleet Subnet Root effects.
//! Boundary: immutable plan/topology/artifact intent is durable before creation, and every
//! uncertain external effect remains fenced in an explicit in-flight phase.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{
        RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
        lock_regular_file_with_parents, read_optional_regular_bytes, write_bytes,
    },
    fleet_install_plan::{PersistedFleetInstallPlan, PlannedCanisterCreationFunding},
    release_set::{
        CanicInfrastructureArtifactEntry, CanicInfrastructureRole,
        PersistedCanicInfrastructureArtifactManifest,
    },
};
use candid::Principal;
use canic_core::{
    bootstrap::compiled::ComponentTopology,
    cdk::utils::hash::decode_hex,
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::fleet_registry::{FleetRegistryManifest, FleetRegistryVersion},
    ids::{
        FleetBinding, FleetCoordinatorBinding, FleetRegistryAuthority, ReleaseBuildId, SubnetId,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const COORDINATOR_INSTALL_JOURNAL_FILE: &str = "coordinator-install-journal.json";
const COORDINATOR_INSTALL_JOURNAL_LOCK_FILE: &str = "coordinator-install-journal.lock";
const COORDINATOR_CREATE_RESULT_FILE: &str = "coordinator-create-result.json";
const COORDINATOR_INSTALL_JOURNAL_SCHEMA_VERSION: u32 = 1;
const MAX_COORDINATOR_INSTALL_JOURNAL_BYTES: usize = 4_194_304;

///
/// FleetCoordinatorInstallPhase
///
/// A distinct in-flight phase prevents uncertain paid effects from being blindly replayed.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum FleetCoordinatorInstallPhase {
    Planned,
    CreationInFlight,
    Created,
    InstallInFlight,
    Installed,
    Verified,
}

///
/// FleetCoordinatorInstallJournal
///
/// Canonical immutable authority plus monotonic observed Coordinator evidence.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetCoordinatorInstallJournal {
    pub schema_version: u32,
    pub sequence: u64,
    pub phase: FleetCoordinatorInstallPhase,
    pub fleet_install_plan_digest: [u8; 32],
    pub infrastructure_manifest_digest: [u8; 32],
    pub fleet: FleetBinding,
    pub release_build_id: ReleaseBuildId,
    pub coordinator_subnet: SubnetId,
    pub creation_funding: PlannedCanisterCreationFunding,
    pub component_topology: ComponentTopology,
    pub coordinator_artifact: CanicInfrastructureArtifactEntry,
    pub expected_module_hash: [u8; 32],
    pub installation_controller: Option<Principal>,
    pub coordinator: Option<Principal>,
    pub installed_module_hash: Option<[u8; 32]>,
    pub verified_registry_manifest: Option<FleetRegistryManifest>,
    pub verified_registry_version: Option<FleetRegistryVersion>,
}

///
/// ResolvedFleetCoordinatorInstall
///
/// One validated durable journal and whether this call advanced its phase.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ResolvedFleetCoordinatorInstall {
    pub journal: FleetCoordinatorInstallJournal,
    pub path: PathBuf,
    pub advanced: bool,
}

///
/// PlanFleetCoordinatorInstallRequest
///

pub(super) struct PlanFleetCoordinatorInstallRequest<'a> {
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub infrastructure_manifest: &'a PersistedCanicInfrastructureArtifactManifest,
    pub component_topology: ComponentTopology,
}

///
/// FleetCoordinatorInstallJournalError
///

#[derive(Debug, ThisError)]
pub(super) enum FleetCoordinatorInstallJournalError {
    #[error(
        "Coordinator install journal already exists with different immutable authority: {path}"
    )]
    ConflictingAuthority { path: PathBuf },

    #[error("Coordinator infrastructure artifact entry is missing")]
    CoordinatorArtifactMissing,

    #[error("Coordinator infrastructure artifact SHA-256 is not one 32-byte digest")]
    InvalidCoordinatorArtifactDigest,

    #[error("invalid Coordinator install journal {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error("Coordinator install journal cannot transition from {observed:?} to {requested:?}")]
    InvalidTransition {
        observed: FleetCoordinatorInstallPhase,
        requested: FleetCoordinatorInstallPhase,
    },

    #[error("failed to access Coordinator install journal {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Coordinator install journal is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error("Coordinator install journal lock is not a regular no-follow file: {path}")]
    UnsafeLock { path: PathBuf },
}

/// Publish or recover the exact pre-effect Coordinator installation authority.
pub(super) fn plan_fleet_coordinator_install(
    request: PlanFleetCoordinatorInstallRequest<'_>,
) -> Result<ResolvedFleetCoordinatorInstall, FleetCoordinatorInstallJournalError> {
    let plan = request.fleet_install_plan;
    let path = coordinator_install_journal_path(&plan.path);
    let _lock = lock_journal(&path)?;
    let expected = planned_journal(&request)?;

    if let Some(observed) = load_optional_journal(&path)? {
        if same_immutable_authority(&observed, &expected) {
            return Ok(resolved(observed, path, false));
        }
        return Err(FleetCoordinatorInstallJournalError::ConflictingAuthority { path });
    }

    let bytes = encode_journal(&path, &expected)?;
    if let Err(source) = create_new_bytes_with_parents(&path, &bytes) {
        match load_optional_journal(&path)? {
            Some(observed) if same_immutable_authority(&observed, &expected) => {
                return Ok(resolved(observed, path, false));
            }
            Some(_) if source.kind() == io::ErrorKind::AlreadyExists => {
                return Err(FleetCoordinatorInstallJournalError::ConflictingAuthority { path });
            }
            _ => {
                return Err(FleetCoordinatorInstallJournalError::Io { path, source });
            }
        }
    }

    let durable = load_required_journal(&path)?;
    if durable != expected {
        return Err(invalid(
            &path,
            "published journal differs from the planned authority",
        ));
    }
    Ok(resolved(durable, path, true))
}

/// Record durable intent immediately before the one Coordinator creation effect.
pub(super) fn begin_coordinator_creation(
    current: &ResolvedFleetCoordinatorInstall,
    installation_controller: Principal,
) -> Result<ResolvedFleetCoordinatorInstall, FleetCoordinatorInstallJournalError> {
    if current.journal.phase == FleetCoordinatorInstallPhase::CreationInFlight
        && current.journal.installation_controller == Some(installation_controller)
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetCoordinatorInstallPhase::Planned,
        FleetCoordinatorInstallPhase::CreationInFlight,
        |next| next.installation_controller = Some(installation_controller),
    )
}

/// Record the real principal returned by the one journalled creation effect.
pub(super) fn record_coordinator_created(
    current: &ResolvedFleetCoordinatorInstall,
    coordinator: Principal,
) -> Result<ResolvedFleetCoordinatorInstall, FleetCoordinatorInstallJournalError> {
    if current.journal.phase == FleetCoordinatorInstallPhase::Created
        && current.journal.coordinator == Some(coordinator)
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetCoordinatorInstallPhase::CreationInFlight,
        FleetCoordinatorInstallPhase::Created,
        |next| next.coordinator = Some(coordinator),
    )
}

/// Record durable intent immediately before installing the exact Coordinator Wasm.
pub(super) fn begin_coordinator_install(
    current: &ResolvedFleetCoordinatorInstall,
) -> Result<ResolvedFleetCoordinatorInstall, FleetCoordinatorInstallJournalError> {
    advance_without_evidence(
        current,
        FleetCoordinatorInstallPhase::Created,
        FleetCoordinatorInstallPhase::InstallInFlight,
    )
}

/// Record independently observed exact Coordinator module identity.
pub(super) fn record_coordinator_installed(
    current: &ResolvedFleetCoordinatorInstall,
    observed_module_hash: [u8; 32],
) -> Result<ResolvedFleetCoordinatorInstall, FleetCoordinatorInstallJournalError> {
    if observed_module_hash != current.journal.expected_module_hash {
        return Err(invalid(
            &current.path,
            "observed installed module hash differs from immutable artifact authority",
        ));
    }
    if current.journal.phase == FleetCoordinatorInstallPhase::Installed
        && current.journal.installed_module_hash == Some(observed_module_hash)
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetCoordinatorInstallPhase::InstallInFlight,
        FleetCoordinatorInstallPhase::Installed,
        |next| next.installed_module_hash = Some(observed_module_hash),
    )
}

/// Record exact live Registry genesis evidence after all three queries agree.
pub(super) fn record_coordinator_verified(
    current: &ResolvedFleetCoordinatorInstall,
    manifest: FleetRegistryManifest,
    version: FleetRegistryVersion,
) -> Result<ResolvedFleetCoordinatorInstall, FleetCoordinatorInstallJournalError> {
    if current.journal.phase == FleetCoordinatorInstallPhase::Verified
        && current.journal.verified_registry_manifest.as_ref() == Some(&manifest)
        && current.journal.verified_registry_version.as_ref() == Some(&version)
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetCoordinatorInstallPhase::Installed,
        FleetCoordinatorInstallPhase::Verified,
        |next| {
            next.verified_registry_manifest = Some(manifest);
            next.verified_registry_version = Some(version);
        },
    )
}

/// Return the durable stdout target for the one Coordinator creation effect.
#[must_use]
pub(super) fn coordinator_create_result_path(plan_path: &Path) -> PathBuf {
    plan_directory(plan_path).join(COORDINATOR_CREATE_RESULT_FILE)
}

fn planned_journal(
    request: &PlanFleetCoordinatorInstallRequest<'_>,
) -> Result<FleetCoordinatorInstallJournal, FleetCoordinatorInstallJournalError> {
    let plan = &request.fleet_install_plan.plan;
    let coordinator_artifact = request
        .infrastructure_manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetCoordinator)
        .cloned()
        .ok_or(FleetCoordinatorInstallJournalError::CoordinatorArtifactMissing)?;
    let expected_module_hash = decode_hex(&coordinator_artifact.wasm_sha256_hex)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or(FleetCoordinatorInstallJournalError::InvalidCoordinatorArtifactDigest)?;
    let journal = FleetCoordinatorInstallJournal {
        schema_version: COORDINATOR_INSTALL_JOURNAL_SCHEMA_VERSION,
        sequence: 0,
        phase: FleetCoordinatorInstallPhase::Planned,
        fleet_install_plan_digest: request.fleet_install_plan.digest,
        infrastructure_manifest_digest: request.infrastructure_manifest.digest,
        fleet: plan.fleet.clone(),
        release_build_id: plan.release_build_id,
        coordinator_subnet: plan.coordinator.coordinator_subnet,
        creation_funding: plan.coordinator.creation_funding.clone(),
        component_topology: request.component_topology.clone(),
        coordinator_artifact,
        expected_module_hash,
        installation_controller: None,
        coordinator: None,
        installed_module_hash: None,
        verified_registry_manifest: None,
        verified_registry_version: None,
    };
    validate_journal(
        &coordinator_install_journal_path(&request.fleet_install_plan.path),
        &journal,
    )?;
    Ok(journal)
}

fn advance_without_evidence(
    current: &ResolvedFleetCoordinatorInstall,
    expected: FleetCoordinatorInstallPhase,
    requested: FleetCoordinatorInstallPhase,
) -> Result<ResolvedFleetCoordinatorInstall, FleetCoordinatorInstallJournalError> {
    if current.journal.phase == requested {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(current, expected, requested, |_| {})
}

fn transition(
    current: &ResolvedFleetCoordinatorInstall,
    expected: FleetCoordinatorInstallPhase,
    requested: FleetCoordinatorInstallPhase,
    apply: impl FnOnce(&mut FleetCoordinatorInstallJournal),
) -> Result<ResolvedFleetCoordinatorInstall, FleetCoordinatorInstallJournalError> {
    if current.journal.phase != expected {
        return Err(FleetCoordinatorInstallJournalError::InvalidTransition {
            observed: current.journal.phase,
            requested,
        });
    }
    let _lock = lock_journal(&current.path)?;
    let observed = load_required_journal(&current.path)?;
    if observed != current.journal {
        return Err(invalid(
            &current.path,
            "journal changed before the requested transition",
        ));
    }

    let mut next = observed;
    next.sequence = next
        .sequence
        .checked_add(1)
        .ok_or_else(|| invalid(&current.path, "journal sequence exhausted"))?;
    next.phase = requested;
    apply(&mut next);
    let bytes = encode_journal(&current.path, &next)?;
    if let Err(source) = write_bytes(&current.path, &bytes) {
        match load_optional_journal(&current.path)? {
            Some(observed) if observed == next => {
                return Ok(resolved(next, current.path.clone(), true));
            }
            _ => {
                return Err(FleetCoordinatorInstallJournalError::Io {
                    path: current.path.clone(),
                    source,
                });
            }
        }
    }
    let durable = load_required_journal(&current.path)?;
    if durable != next {
        return Err(invalid(
            &current.path,
            "published transition differs from the requested journal",
        ));
    }
    Ok(resolved(durable, current.path.clone(), true))
}

fn load_required_journal(
    path: &Path,
) -> Result<FleetCoordinatorInstallJournal, FleetCoordinatorInstallJournalError> {
    load_optional_journal(path)?.ok_or_else(|| invalid(path, "journal is missing"))
}

fn load_optional_journal(
    path: &Path,
) -> Result<Option<FleetCoordinatorInstallJournal>, FleetCoordinatorInstallJournalError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(bytes) => bytes,
        Err(RegularFileReadError::NotRegular) => {
            return Err(FleetCoordinatorInstallJournalError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(FleetCoordinatorInstallJournalError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(FleetCoordinatorInstallJournalError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Coordinator install journal reads are unsupported",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    if bytes.len() > MAX_COORDINATOR_INSTALL_JOURNAL_BYTES {
        return Err(invalid(path, "journal exceeds its byte bound"));
    }
    let journal = serde_json::from_slice::<FleetCoordinatorInstallJournal>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    validate_journal(path, &journal)?;
    if encode_journal(path, &journal)? != bytes {
        return Err(invalid(path, "journal bytes are not canonical"));
    }
    Ok(Some(journal))
}

fn validate_journal(
    path: &Path,
    journal: &FleetCoordinatorInstallJournal,
) -> Result<(), FleetCoordinatorInstallJournalError> {
    validate_immutable_authority(path, journal)?;
    validate_phase_evidence(path, journal)?;
    validate_registry_evidence(path, journal)
}

fn validate_immutable_authority(
    path: &Path,
    journal: &FleetCoordinatorInstallJournal,
) -> Result<(), FleetCoordinatorInstallJournalError> {
    if journal.schema_version != COORDINATOR_INSTALL_JOURNAL_SCHEMA_VERSION {
        return Err(invalid(path, "unsupported journal schema version"));
    }
    journal
        .component_topology
        .canonical_bytes()
        .map_err(|error| invalid(path, error.to_string()))?;
    if journal.coordinator_subnet.as_principal() == &Principal::anonymous() {
        return Err(invalid(path, "Coordinator Subnet is anonymous"));
    }
    if journal
        .installation_controller
        .is_some_and(|controller| controller == Principal::anonymous())
    {
        return Err(invalid(path, "installation controller is anonymous"));
    }
    let funding_is_positive = match journal.creation_funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => cycles > 0,
        PlannedCanisterCreationFunding::Icp { e8s } => e8s > 0,
    };
    if !funding_is_positive {
        return Err(invalid(
            path,
            "Coordinator creation funding is not positive",
        ));
    }
    if journal.coordinator_artifact.role != CanicInfrastructureRole::FleetCoordinator
        || journal.coordinator_artifact.release_build_id != journal.release_build_id
    {
        return Err(invalid(
            path,
            "Coordinator artifact does not match the journalled role and release build",
        ));
    }
    let artifact_hash = decode_hex(&journal.coordinator_artifact.wasm_sha256_hex)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| invalid(path, "Coordinator artifact digest is invalid"))?;
    if journal.expected_module_hash != artifact_hash {
        return Err(invalid(
            path,
            "expected module hash does not match the Coordinator artifact",
        ));
    }
    Ok(())
}

fn validate_phase_evidence(
    path: &Path,
    journal: &FleetCoordinatorInstallJournal,
) -> Result<(), FleetCoordinatorInstallJournalError> {
    let expected_sequence = match journal.phase {
        FleetCoordinatorInstallPhase::Planned => 0,
        FleetCoordinatorInstallPhase::CreationInFlight => 1,
        FleetCoordinatorInstallPhase::Created => 2,
        FleetCoordinatorInstallPhase::InstallInFlight => 3,
        FleetCoordinatorInstallPhase::Installed => 4,
        FleetCoordinatorInstallPhase::Verified => 5,
    };
    if journal.sequence != expected_sequence {
        return Err(invalid(path, "phase does not match journal sequence"));
    }
    let has_controller = journal.installation_controller.is_some();
    let has_coordinator = journal.coordinator.is_some();
    let has_installed = journal.installed_module_hash.is_some();
    let has_manifest = journal.verified_registry_manifest.is_some();
    let has_version = journal.verified_registry_version.is_some();
    let valid_evidence = match journal.phase {
        FleetCoordinatorInstallPhase::Planned => {
            !has_controller && !has_coordinator && !has_installed && !has_manifest && !has_version
        }
        FleetCoordinatorInstallPhase::CreationInFlight => {
            has_controller && !has_coordinator && !has_installed && !has_manifest && !has_version
        }
        FleetCoordinatorInstallPhase::Created | FleetCoordinatorInstallPhase::InstallInFlight => {
            has_controller && has_coordinator && !has_installed && !has_manifest && !has_version
        }
        FleetCoordinatorInstallPhase::Installed => {
            has_controller && has_coordinator && has_installed && !has_manifest && !has_version
        }
        FleetCoordinatorInstallPhase::Verified => {
            has_controller && has_coordinator && has_installed && has_manifest && has_version
        }
    };
    if !valid_evidence {
        return Err(invalid(
            path,
            "phase does not match retained Coordinator evidence",
        ));
    }
    if journal.installed_module_hash.is_some()
        && journal.installed_module_hash != Some(journal.expected_module_hash)
    {
        return Err(invalid(
            path,
            "installed module hash differs from immutable artifact authority",
        ));
    }
    Ok(())
}

fn validate_registry_evidence(
    path: &Path,
    journal: &FleetCoordinatorInstallJournal,
) -> Result<(), FleetCoordinatorInstallJournalError> {
    if let Some(coordinator) = journal.coordinator {
        if coordinator == Principal::anonymous() {
            return Err(invalid(path, "Coordinator principal is anonymous"));
        }
        let authority = FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: journal.fleet.clone(),
                coordinator_subnet: journal.coordinator_subnet,
                coordinator,
            },
            epoch: 1,
        };
        let registry = FleetRegistryOps::compile_genesis(
            &journal.fleet.app,
            authority.clone(),
            &journal.component_topology,
        )
        .map_err(|error| invalid(path, error.to_string()))?;
        let expected_manifest =
            FleetRegistryOps::manifest(&authority, &journal.component_topology, &registry)
                .map_err(|error| invalid(path, error.to_string()))?;
        let expected_version =
            FleetRegistryOps::version(&authority, &journal.component_topology, &registry)
                .map_err(|error| invalid(path, error.to_string()))?;
        if journal.phase == FleetCoordinatorInstallPhase::Verified
            && (journal.verified_registry_manifest.as_ref() != Some(&expected_manifest)
                || journal.verified_registry_version.as_ref() != Some(&expected_version))
        {
            return Err(invalid(
                path,
                "verified Registry evidence differs from exact Coordinator genesis",
            ));
        }
    }
    Ok(())
}

fn encode_journal(
    path: &Path,
    journal: &FleetCoordinatorInstallJournal,
) -> Result<Vec<u8>, FleetCoordinatorInstallJournalError> {
    validate_journal(path, journal)?;
    let bytes = serde_json::to_vec(journal).map_err(|error| invalid(path, error.to_string()))?;
    if bytes.len() > MAX_COORDINATOR_INSTALL_JOURNAL_BYTES {
        return Err(invalid(path, "journal exceeds its byte bound"));
    }
    Ok(bytes)
}

fn same_immutable_authority(
    observed: &FleetCoordinatorInstallJournal,
    expected: &FleetCoordinatorInstallJournal,
) -> bool {
    observed.schema_version == expected.schema_version
        && observed.fleet_install_plan_digest == expected.fleet_install_plan_digest
        && observed.infrastructure_manifest_digest == expected.infrastructure_manifest_digest
        && observed.fleet == expected.fleet
        && observed.release_build_id == expected.release_build_id
        && observed.coordinator_subnet == expected.coordinator_subnet
        && observed.creation_funding == expected.creation_funding
        && observed.component_topology == expected.component_topology
        && observed.coordinator_artifact == expected.coordinator_artifact
        && observed.expected_module_hash == expected.expected_module_hash
}

fn coordinator_install_journal_path(plan_path: &Path) -> PathBuf {
    plan_directory(plan_path).join(COORDINATOR_INSTALL_JOURNAL_FILE)
}

fn plan_directory(plan_path: &Path) -> &Path {
    plan_path
        .parent()
        .expect("validated Fleet install plan path has an identity directory")
}

fn lock_journal(path: &Path) -> Result<std::fs::File, FleetCoordinatorInstallJournalError> {
    let lock_path = path.with_file_name(COORDINATOR_INSTALL_JOURNAL_LOCK_FILE);
    match lock_regular_file_with_parents(&lock_path) {
        Ok(lock) => Ok(lock),
        Err(RegularFileLockError::NotRegular) => {
            Err(FleetCoordinatorInstallJournalError::UnsafeLock { path: lock_path })
        }
        Err(RegularFileLockError::Io(source)) => Err(FleetCoordinatorInstallJournalError::Io {
            path: lock_path,
            source,
        }),
        #[cfg(windows)]
        Err(RegularFileLockError::UnsupportedPlatform) => {
            Err(FleetCoordinatorInstallJournalError::Io {
                path: lock_path,
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Coordinator install journal locking is unsupported",
                ),
            })
        }
    }
}

const fn resolved(
    journal: FleetCoordinatorInstallJournal,
    path: PathBuf,
    advanced: bool,
) -> ResolvedFleetCoordinatorInstall {
    ResolvedFleetCoordinatorInstall {
        journal,
        path,
        advanced,
    }
}

fn invalid(path: &Path, reason: impl Into<String>) -> FleetCoordinatorInstallJournalError {
    FleetCoordinatorInstallJournalError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
