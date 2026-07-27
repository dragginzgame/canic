//! Module: install_root::fleet_subnet_root_install_journal
//!
//! Responsibility: own canonical host recovery authority for one planned Fleet Subnet Root.
//! Does not own: ICP commands, artifact reads, Store publication effects, or Registry mutation.
//! Boundary: every root and Store effect has a durable phase and exact retry never changes the
//! Fleet plan, Coordinator authority, root placement, release set, or install operation.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{
        RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
        lock_regular_file_with_parents, read_optional_regular_bytes, write_bytes,
    },
    fleet_install_plan::{PersistedFleetInstallPlan, PlannedFleetSubnetRoot},
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
    dto::{
        fleet_registry::{
            FleetRegistry, FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootEntry,
            FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse, FleetSubnetRootStatus,
        },
        fleet_subnet_root::FleetSubnetRootAuthority,
        root_store::RootStoreBootstrapResponse,
    },
    ids::{
        FleetCoordinatorBinding, FleetRegistryAuthority, FleetSubnetRootBinding, ReleaseBuildId,
        SubnetId,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const JOURNAL_FILE: &str = "install-journal.json";
const JOURNAL_LOCK_FILE: &str = "install-journal.lock";
const CREATE_RESULT_FILE: &str = "create-result.json";
const ROOT_INSTALL_DIRECTORY: &str = "fleet-subnet-root-installs";
const JOURNAL_SCHEMA_VERSION: u32 = 3;
const MAX_JOURNAL_BYTES: usize = 4_194_304;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum FleetSubnetRootInstallPhase {
    Planned,
    CreationInFlight,
    Created,
    InstallInFlight,
    Installed,
    Verified,
    StoreStaging,
    StoreStaged,
    StoreBootstrapInFlight,
    StoreBootstrapped,
    StoreVerified,
    RegistryJoinInFlight,
    RegistryJoined,
    RegistryJoinVerified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FleetSubnetRootInstallJournal {
    pub schema_version: u32,
    pub sequence: u64,
    pub phase: FleetSubnetRootInstallPhase,
    pub fleet_install_plan_digest: [u8; 32],
    pub infrastructure_manifest_digest: [u8; 32],
    pub authority: FleetRegistryAuthority,
    pub release_build_id: ReleaseBuildId,
    pub install_operation_id: [u8; 32],
    pub component_topology: ComponentTopology,
    pub root_plan: PlannedFleetSubnetRoot,
    pub root_artifact: CanicInfrastructureArtifactEntry,
    pub expected_module_hash: [u8; 32],
    pub fleet_subnet_root: Option<Principal>,
    pub installed_module_hash: Option<[u8; 32]>,
    pub verified_binding: Option<FleetSubnetRootBinding>,
    pub store_bootstrap: Option<RootStoreBootstrapResponse>,
    pub registry_join_request: Option<FleetSubnetRootJoinRequest>,
    pub registry_join_response: Option<FleetSubnetRootJoinResponse>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ResolvedFleetSubnetRootInstall {
    pub journal: FleetSubnetRootInstallJournal,
    pub path: PathBuf,
    pub advanced: bool,
}

pub(super) struct PlanFleetSubnetRootInstallRequest<'a> {
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub infrastructure_manifest: &'a PersistedCanicInfrastructureArtifactManifest,
    pub coordinator: Principal,
    pub install_operation_id: [u8; 32],
    pub component_topology: ComponentTopology,
    pub root_plan: &'a PlannedFleetSubnetRoot,
}

#[derive(Debug, ThisError)]
pub(super) enum FleetSubnetRootInstallJournalError {
    #[error("Fleet Subnet Root install journal already has different authority: {path}")]
    ConflictingAuthority { path: PathBuf },

    #[error("Fleet Subnet Root infrastructure artifact entry is missing")]
    RootArtifactMissing,

    #[error("Fleet Subnet Root artifact SHA-256 is not one 32-byte digest")]
    InvalidRootArtifactDigest,

    #[error("invalid Fleet Subnet Root install journal {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error(
        "Fleet Subnet Root install journal cannot transition from {observed:?} to {requested:?}"
    )]
    InvalidTransition {
        observed: FleetSubnetRootInstallPhase,
        requested: FleetSubnetRootInstallPhase,
    },

    #[error("failed to access Fleet Subnet Root install journal {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Fleet Subnet Root install journal is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error("Fleet Subnet Root install journal lock is not a regular no-follow file: {path}")]
    UnsafeLock { path: PathBuf },
}

pub(super) fn plan_fleet_subnet_root_install(
    request: PlanFleetSubnetRootInstallRequest<'_>,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    let path = journal_path(
        &request.fleet_install_plan.path,
        request.root_plan.placement_subnet,
    );
    let _lock = lock_journal(&path)?;
    let expected = planned_journal(&request)?;

    if let Some(observed) = load_optional_journal(&path)? {
        if same_immutable_authority(&observed, &expected) {
            return Ok(resolved(observed, path, false));
        }
        return Err(FleetSubnetRootInstallJournalError::ConflictingAuthority { path });
    }

    let bytes = encode_journal(&path, &expected)?;
    if let Err(source) = create_new_bytes_with_parents(&path, &bytes) {
        match load_optional_journal(&path)? {
            Some(observed) if same_immutable_authority(&observed, &expected) => {
                return Ok(resolved(observed, path, false));
            }
            Some(_) if source.kind() == io::ErrorKind::AlreadyExists => {
                return Err(FleetSubnetRootInstallJournalError::ConflictingAuthority { path });
            }
            _ => return Err(FleetSubnetRootInstallJournalError::Io { path, source }),
        }
    }

    let durable = load_required_journal(&path)?;
    if durable != expected {
        return Err(invalid(
            &path,
            "published journal differs from planned root authority",
        ));
    }
    Ok(resolved(durable, path, true))
}

pub(super) fn begin_root_creation(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::Planned,
        FleetSubnetRootInstallPhase::CreationInFlight,
    )
}

pub(super) fn record_root_created(
    current: &ResolvedFleetSubnetRootInstall,
    fleet_subnet_root: Principal,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    if current.journal.phase == FleetSubnetRootInstallPhase::Created
        && current.journal.fleet_subnet_root == Some(fleet_subnet_root)
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetSubnetRootInstallPhase::CreationInFlight,
        FleetSubnetRootInstallPhase::Created,
        |next| next.fleet_subnet_root = Some(fleet_subnet_root),
    )
}

pub(super) fn begin_root_install(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::Created,
        FleetSubnetRootInstallPhase::InstallInFlight,
    )
}

pub(super) fn record_root_installed(
    current: &ResolvedFleetSubnetRootInstall,
    observed_module_hash: [u8; 32],
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    if observed_module_hash != current.journal.expected_module_hash {
        return Err(invalid(
            &current.path,
            "observed root module differs from immutable artifact authority",
        ));
    }
    if current.journal.phase == FleetSubnetRootInstallPhase::Installed
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
        FleetSubnetRootInstallPhase::InstallInFlight,
        FleetSubnetRootInstallPhase::Installed,
        |next| next.installed_module_hash = Some(observed_module_hash),
    )
}

pub(super) fn record_root_verified(
    current: &ResolvedFleetSubnetRootInstall,
    authority: FleetSubnetRootAuthority,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    let expected = expected_root_authority(&current.journal)?;
    if authority != expected {
        return Err(invalid(
            &current.path,
            "observed root authority differs from exact planned binding",
        ));
    }
    if current.journal.phase == FleetSubnetRootInstallPhase::Verified
        && current.journal.verified_binding.as_ref() == Some(&authority.binding)
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetSubnetRootInstallPhase::Installed,
        FleetSubnetRootInstallPhase::Verified,
        |next| next.verified_binding = Some(authority.binding),
    )
}

pub(super) fn begin_store_staging(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::Verified,
        FleetSubnetRootInstallPhase::StoreStaging,
    )
}

pub(super) fn record_store_staged(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::StoreStaging,
        FleetSubnetRootInstallPhase::StoreStaged,
    )
}

pub(super) fn begin_store_bootstrap(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::StoreStaged,
        FleetSubnetRootInstallPhase::StoreBootstrapInFlight,
    )
}

pub(super) fn record_store_bootstrapped(
    current: &ResolvedFleetSubnetRootInstall,
    evidence: RootStoreBootstrapResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_store_bootstrap_evidence(&current.path, &current.journal, &evidence)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::StoreBootstrapInFlight,
        FleetSubnetRootInstallPhase::StoreBootstrapped,
        |next| next.store_bootstrap = Some(evidence),
    )
}

pub(super) fn record_store_verified(
    current: &ResolvedFleetSubnetRootInstall,
    evidence: RootStoreBootstrapResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_store_bootstrap_evidence(&current.path, &current.journal, &evidence)?;
    if current.journal.store_bootstrap.as_ref() != Some(&evidence) {
        return Err(invalid(
            &current.path,
            "verified Store evidence differs from bootstrap result",
        ));
    }
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::StoreBootstrapped,
        FleetSubnetRootInstallPhase::StoreVerified,
    )
}

pub(super) fn begin_registry_join(
    current: &ResolvedFleetSubnetRootInstall,
    expected_registry: FleetRegistryVersion,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    let request = FleetSubnetRootJoinRequest {
        expected_registry,
        entry: expected_registry_join_entry(&current.journal)?,
    };
    validate_registry_join_request(&current.path, &current.journal, &request)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::StoreVerified,
        FleetSubnetRootInstallPhase::RegistryJoinInFlight,
        |next| next.registry_join_request = Some(request),
    )
}

pub(super) fn record_registry_joined(
    current: &ResolvedFleetSubnetRootInstall,
    response: FleetSubnetRootJoinResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_registry_join_response(&current.path, &current.journal, &response)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::RegistryJoinInFlight,
        FleetSubnetRootInstallPhase::RegistryJoined,
        |next| next.registry_join_response = Some(response),
    )
}

pub(super) fn record_registry_join_verified(
    current: &ResolvedFleetSubnetRootInstall,
    registry: &FleetRegistry,
    manifest: &FleetRegistryManifest,
    version: &FleetRegistryVersion,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_joined_registry_snapshot(
        &current.path,
        &current.journal,
        registry,
        manifest,
        version,
    )?;
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::RegistryJoined,
        FleetSubnetRootInstallPhase::RegistryJoinVerified,
    )
}

#[must_use]
pub(super) fn create_result_path(journal_path: &Path) -> PathBuf {
    journal_path
        .parent()
        .expect("validated root journal has an identity directory")
        .join(CREATE_RESULT_FILE)
}

pub(super) fn expected_root_authority(
    journal: &FleetSubnetRootInstallJournal,
) -> Result<FleetSubnetRootAuthority, FleetSubnetRootInstallJournalError> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .ok_or_else(|| invalid(Path::new(JOURNAL_FILE), "root principal is missing"))?;
    let authority = FleetSubnetRootAuthority {
        binding: FleetSubnetRootBinding {
            authority: journal.authority.clone(),
            placement_subnet: journal.root_plan.placement_subnet,
            fleet_subnet_root,
            component_admissions: journal.root_plan.component_admissions.clone(),
            component_topology_digest: journal.root_plan.component_topology_digest,
            limits: journal.root_plan.limits.clone(),
        },
        initial_release_set: journal.root_plan.initial_release_set,
        expected_module_hash: journal.expected_module_hash,
    };
    journal
        .component_topology
        .validate_root_binding(&authority.binding)
        .map_err(|error| invalid(Path::new(JOURNAL_FILE), error.to_string()))?;
    Ok(authority)
}

pub(super) fn expected_registry_join_entry(
    journal: &FleetSubnetRootInstallJournal,
) -> Result<FleetSubnetRootEntry, FleetSubnetRootInstallJournalError> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .ok_or_else(|| invalid(Path::new(JOURNAL_FILE), "root principal is missing"))?;
    Ok(FleetSubnetRootEntry {
        placement_subnet: journal.root_plan.placement_subnet,
        fleet_subnet_root,
        component_admissions: journal.root_plan.component_admissions.clone(),
        component_topology_digest: journal.root_plan.component_topology_digest,
        active_release_set: journal.root_plan.initial_release_set,
        limits: journal.root_plan.limits.clone(),
        status: FleetSubnetRootStatus::Joining,
    })
}

fn planned_journal(
    request: &PlanFleetSubnetRootInstallRequest<'_>,
) -> Result<FleetSubnetRootInstallJournal, FleetSubnetRootInstallJournalError> {
    let plan = &request.fleet_install_plan.plan;
    let path = journal_path(
        &request.fleet_install_plan.path,
        request.root_plan.placement_subnet,
    );
    if !plan.fleet_subnet_roots.contains(request.root_plan) {
        return Err(invalid(
            &path,
            "root plan is not a member of the immutable Fleet install plan",
        ));
    }
    let root_artifact = request
        .infrastructure_manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .cloned()
        .ok_or(FleetSubnetRootInstallJournalError::RootArtifactMissing)?;
    let expected_module_hash = decode_hex(&root_artifact.wasm_sha256_hex)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or(FleetSubnetRootInstallJournalError::InvalidRootArtifactDigest)?;
    let journal = FleetSubnetRootInstallJournal {
        schema_version: JOURNAL_SCHEMA_VERSION,
        sequence: 0,
        phase: FleetSubnetRootInstallPhase::Planned,
        fleet_install_plan_digest: request.fleet_install_plan.digest,
        infrastructure_manifest_digest: request.infrastructure_manifest.digest,
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: plan.fleet.clone(),
                coordinator_subnet: plan.coordinator.coordinator_subnet,
                coordinator: request.coordinator,
            },
            epoch: 1,
        },
        release_build_id: plan.release_build_id,
        install_operation_id: request.install_operation_id,
        component_topology: request.component_topology.clone(),
        root_plan: request.root_plan.clone(),
        root_artifact,
        expected_module_hash,
        fleet_subnet_root: None,
        installed_module_hash: None,
        verified_binding: None,
        store_bootstrap: None,
        registry_join_request: None,
        registry_join_response: None,
    };
    validate_journal(&path, &journal)?;
    Ok(journal)
}

fn advance_without_evidence(
    current: &ResolvedFleetSubnetRootInstall,
    expected: FleetSubnetRootInstallPhase,
    requested: FleetSubnetRootInstallPhase,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
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
    current: &ResolvedFleetSubnetRootInstall,
    expected: FleetSubnetRootInstallPhase,
    requested: FleetSubnetRootInstallPhase,
    apply: impl FnOnce(&mut FleetSubnetRootInstallJournal),
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    if current.journal.phase != expected {
        return Err(FleetSubnetRootInstallJournalError::InvalidTransition {
            observed: current.journal.phase,
            requested,
        });
    }
    let _lock = lock_journal(&current.path)?;
    let observed = load_required_journal(&current.path)?;
    if observed != current.journal {
        return Err(invalid(
            &current.path,
            "journal changed before requested transition",
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
                return Err(FleetSubnetRootInstallJournalError::Io {
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
            "published transition differs from requested journal",
        ));
    }
    Ok(resolved(durable, current.path.clone(), true))
}

fn load_required_journal(
    path: &Path,
) -> Result<FleetSubnetRootInstallJournal, FleetSubnetRootInstallJournalError> {
    load_optional_journal(path)?.ok_or_else(|| invalid(path, "journal is missing"))
}

fn load_optional_journal(
    path: &Path,
) -> Result<Option<FleetSubnetRootInstallJournal>, FleetSubnetRootInstallJournalError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(bytes) => bytes,
        Err(RegularFileReadError::NotRegular) => {
            return Err(FleetSubnetRootInstallJournalError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(FleetSubnetRootInstallJournalError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(FleetSubnetRootInstallJournalError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Fleet Subnet Root install journal reads are unsupported",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(invalid(path, "journal exceeds its byte bound"));
    }
    let journal = serde_json::from_slice::<FleetSubnetRootInstallJournal>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    validate_journal(path, &journal)?;
    if encode_journal(path, &journal)? != bytes {
        return Err(invalid(path, "journal bytes are not canonical"));
    }
    Ok(Some(journal))
}

fn validate_journal(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    validate_immutable_authority(path, journal)?;
    validate_phase_evidence(path, journal)?;
    validate_root_evidence(path, journal)
}

fn validate_immutable_authority(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    if journal.schema_version != JOURNAL_SCHEMA_VERSION {
        return Err(invalid(path, "unsupported journal schema version"));
    }
    if journal.install_operation_id == [0; 32] {
        return Err(invalid(path, "install operation identity must not be zero"));
    }
    journal
        .component_topology
        .canonical_bytes()
        .map_err(|error| invalid(path, error.to_string()))?;
    if journal.authority.epoch != 1
        || journal.authority.binding.coordinator == Principal::anonymous()
        || journal.authority.binding.coordinator_subnet.as_principal() == &Principal::anonymous()
    {
        return Err(invalid(path, "Coordinator authority is invalid"));
    }
    if journal.root_plan.placement_subnet.as_principal() == &Principal::anonymous() {
        return Err(invalid(path, "root placement Subnet is anonymous"));
    }
    if journal.root_plan.initial_release_set.release_build_id != journal.release_build_id {
        return Err(invalid(
            path,
            "root release set differs from journal release build",
        ));
    }
    journal
        .component_topology
        .validate_planned_root(
            &journal.root_plan.component_admissions,
            journal.root_plan.component_topology_digest,
            &journal.root_plan.limits,
        )
        .map_err(|error| invalid(path, error.to_string()))?;
    if journal.root_artifact.role != CanicInfrastructureRole::FleetSubnetRoot
        || journal.root_artifact.release_build_id != journal.release_build_id
    {
        return Err(invalid(
            path,
            "root artifact differs from role or release build authority",
        ));
    }
    let artifact_hash = decode_hex(&journal.root_artifact.wasm_sha256_hex)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| invalid(path, "root artifact digest is invalid"))?;
    if journal.expected_module_hash != artifact_hash {
        return Err(invalid(
            path,
            "expected module hash differs from root artifact",
        ));
    }
    Ok(())
}

fn validate_phase_evidence(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let expected_sequence = match journal.phase {
        FleetSubnetRootInstallPhase::Planned => 0,
        FleetSubnetRootInstallPhase::CreationInFlight => 1,
        FleetSubnetRootInstallPhase::Created => 2,
        FleetSubnetRootInstallPhase::InstallInFlight => 3,
        FleetSubnetRootInstallPhase::Installed => 4,
        FleetSubnetRootInstallPhase::Verified => 5,
        FleetSubnetRootInstallPhase::StoreStaging => 6,
        FleetSubnetRootInstallPhase::StoreStaged => 7,
        FleetSubnetRootInstallPhase::StoreBootstrapInFlight => 8,
        FleetSubnetRootInstallPhase::StoreBootstrapped => 9,
        FleetSubnetRootInstallPhase::StoreVerified => 10,
        FleetSubnetRootInstallPhase::RegistryJoinInFlight => 11,
        FleetSubnetRootInstallPhase::RegistryJoined => 12,
        FleetSubnetRootInstallPhase::RegistryJoinVerified => 13,
    };
    if journal.sequence != expected_sequence {
        return Err(invalid(path, "phase differs from journal sequence"));
    }
    let has_root = journal.fleet_subnet_root.is_some();
    let has_installed = journal.installed_module_hash.is_some();
    let has_verified = journal.verified_binding.is_some();
    let has_store = journal.store_bootstrap.is_some();
    let has_join_request = journal.registry_join_request.is_some();
    let has_join_response = journal.registry_join_response.is_some();
    let valid = match journal.phase {
        FleetSubnetRootInstallPhase::Planned | FleetSubnetRootInstallPhase::CreationInFlight => {
            !has_root
                && !has_installed
                && !has_verified
                && !has_store
                && !has_join_request
                && !has_join_response
        }
        FleetSubnetRootInstallPhase::Created | FleetSubnetRootInstallPhase::InstallInFlight => {
            has_root
                && !has_installed
                && !has_verified
                && !has_store
                && !has_join_request
                && !has_join_response
        }
        FleetSubnetRootInstallPhase::Installed => {
            has_root
                && has_installed
                && !has_verified
                && !has_store
                && !has_join_request
                && !has_join_response
        }
        FleetSubnetRootInstallPhase::Verified
        | FleetSubnetRootInstallPhase::StoreStaging
        | FleetSubnetRootInstallPhase::StoreStaged
        | FleetSubnetRootInstallPhase::StoreBootstrapInFlight => {
            has_root
                && has_installed
                && has_verified
                && !has_store
                && !has_join_request
                && !has_join_response
        }
        FleetSubnetRootInstallPhase::StoreBootstrapped
        | FleetSubnetRootInstallPhase::StoreVerified => {
            has_root
                && has_installed
                && has_verified
                && has_store
                && !has_join_request
                && !has_join_response
        }
        FleetSubnetRootInstallPhase::RegistryJoinInFlight => {
            has_root
                && has_installed
                && has_verified
                && has_store
                && has_join_request
                && !has_join_response
        }
        FleetSubnetRootInstallPhase::RegistryJoined
        | FleetSubnetRootInstallPhase::RegistryJoinVerified => {
            has_root
                && has_installed
                && has_verified
                && has_store
                && has_join_request
                && has_join_response
        }
    };
    if !valid {
        return Err(invalid(path, "phase differs from retained root evidence"));
    }
    if journal.installed_module_hash.is_some()
        && journal.installed_module_hash != Some(journal.expected_module_hash)
    {
        return Err(invalid(
            path,
            "installed module differs from artifact authority",
        ));
    }
    Ok(())
}

fn validate_root_evidence(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let Some(fleet_subnet_root) = journal.fleet_subnet_root else {
        return Ok(());
    };
    if fleet_subnet_root == Principal::anonymous()
        || fleet_subnet_root == journal.authority.binding.coordinator
    {
        return Err(invalid(
            path,
            "root principal conflicts with Fleet authority",
        ));
    }
    let expected =
        expected_root_authority(journal).map_err(|error| invalid(path, error.to_string()))?;
    if matches!(
        journal.phase,
        FleetSubnetRootInstallPhase::Verified
            | FleetSubnetRootInstallPhase::StoreStaging
            | FleetSubnetRootInstallPhase::StoreStaged
            | FleetSubnetRootInstallPhase::StoreBootstrapInFlight
            | FleetSubnetRootInstallPhase::StoreBootstrapped
            | FleetSubnetRootInstallPhase::StoreVerified
            | FleetSubnetRootInstallPhase::RegistryJoinInFlight
            | FleetSubnetRootInstallPhase::RegistryJoined
            | FleetSubnetRootInstallPhase::RegistryJoinVerified
    ) && journal.verified_binding.as_ref() != Some(&expected.binding)
    {
        return Err(invalid(
            path,
            "verified root authority differs from exact planned binding",
        ));
    }
    if let Some(evidence) = &journal.store_bootstrap {
        validate_store_bootstrap_evidence(path, journal, evidence)?;
    }
    if let Some(request) = &journal.registry_join_request {
        validate_registry_join_request(path, journal, request)?;
    }
    if let Some(response) = &journal.registry_join_response {
        validate_registry_join_response(path, journal, response)?;
    }
    Ok(())
}

fn validate_store_bootstrap_evidence(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    evidence: &RootStoreBootstrapResponse,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .ok_or_else(|| invalid(path, "Store evidence requires a root principal"))?;
    if evidence.fleet_subnet_root != fleet_subnet_root
        || evidence.wasm_store == Principal::anonymous()
        || evidence.wasm_store == fleet_subnet_root
        || evidence.wasm_store == journal.authority.binding.coordinator
        || evidence.release_set != journal.root_plan.initial_release_set
        || evidence.catalog.is_empty()
    {
        return Err(invalid(
            path,
            "Store bootstrap evidence differs from immutable root authority",
        ));
    }
    let projected = journal
        .component_topology
        .project_for_admissions(&journal.root_plan.component_admissions)
        .map_err(|error| invalid(path, error.to_string()))?;
    let mut expected_roles = BTreeSet::new();
    for spec in projected.component_specs {
        expected_roles.insert(spec.component_role);
        expected_roles.extend(spec.children.into_iter().map(|child| child.role));
    }
    let observed_roles = evidence
        .catalog
        .iter()
        .map(|entry| entry.role.clone())
        .collect::<BTreeSet<_>>();
    if observed_roles != expected_roles {
        return Err(invalid(
            path,
            "Store bootstrap catalog roles differ from root admissions",
        ));
    }
    let mut previous = None;
    for entry in &evidence.catalog {
        if entry.payload_size_bytes == 0
            || entry.payload_hash == [0; 32]
            || previous.as_ref().is_some_and(|role| role >= &entry.role)
        {
            return Err(invalid(path, "Store bootstrap catalog is not canonical"));
        }
        previous = Some(entry.role.clone());
    }
    Ok(())
}

fn validate_registry_join_request(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    request: &FleetSubnetRootJoinRequest,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    if request.expected_registry.authority != journal.authority
        || request.expected_registry.revision == 0
        || request.expected_registry.content_hash == [0; 32]
        || request.entry != expected_registry_join_entry(journal)?
    {
        return Err(invalid(
            path,
            "Registry join request differs from immutable root authority",
        ));
    }
    Ok(())
}

fn validate_registry_join_response(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    response: &FleetSubnetRootJoinResponse,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let request = journal
        .registry_join_request
        .as_ref()
        .ok_or_else(|| invalid(path, "Registry join response requires a durable request"))?;
    let expected_revision = request
        .expected_registry
        .revision
        .checked_add(1)
        .ok_or_else(|| invalid(path, "Registry join expected revision is exhausted"))?;
    if response.entry != request.entry
        || response.version.authority != journal.authority
        || response.version.revision != expected_revision
        || response.version.content_hash == [0; 32]
    {
        return Err(invalid(
            path,
            "Registry join response differs from the durable request",
        ));
    }
    Ok(())
}

fn validate_joined_registry_snapshot(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    registry: &FleetRegistry,
    manifest: &FleetRegistryManifest,
    version: &FleetRegistryVersion,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let response = journal.registry_join_response.as_ref().ok_or_else(|| {
        invalid(
            path,
            "Registry verification requires a durable join response",
        )
    })?;
    FleetRegistryOps::validate(&journal.authority, &journal.component_topology, registry)
        .map_err(|error| invalid(path, error.to_string()))?;
    let expected_manifest =
        FleetRegistryOps::manifest(&journal.authority, &journal.component_topology, registry)
            .map_err(|error| invalid(path, error.to_string()))?;
    let expected_version =
        FleetRegistryOps::version(&journal.authority, &journal.component_topology, registry)
            .map_err(|error| invalid(path, error.to_string()))?;
    if manifest != &expected_manifest
        || version != &expected_version
        || version != &response.version
        || !registry
            .fleet_subnet_roots
            .iter()
            .any(|entry| entry == &response.entry)
    {
        return Err(invalid(
            path,
            "live Registry evidence differs from the exact joined snapshot",
        ));
    }
    Ok(())
}

fn encode_journal(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<Vec<u8>, FleetSubnetRootInstallJournalError> {
    validate_journal(path, journal)?;
    let bytes = serde_json::to_vec(journal).map_err(|error| invalid(path, error.to_string()))?;
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(invalid(path, "journal exceeds its byte bound"));
    }
    Ok(bytes)
}

fn same_immutable_authority(
    observed: &FleetSubnetRootInstallJournal,
    expected: &FleetSubnetRootInstallJournal,
) -> bool {
    observed.schema_version == expected.schema_version
        && observed.fleet_install_plan_digest == expected.fleet_install_plan_digest
        && observed.infrastructure_manifest_digest == expected.infrastructure_manifest_digest
        && observed.authority == expected.authority
        && observed.release_build_id == expected.release_build_id
        && observed.install_operation_id == expected.install_operation_id
        && observed.component_topology == expected.component_topology
        && observed.root_plan == expected.root_plan
        && observed.root_artifact == expected.root_artifact
        && observed.expected_module_hash == expected.expected_module_hash
}

fn journal_path(plan_path: &Path, placement_subnet: SubnetId) -> PathBuf {
    plan_path
        .parent()
        .expect("validated Fleet install plan has an identity directory")
        .join(ROOT_INSTALL_DIRECTORY)
        .join(placement_subnet.to_string())
        .join(JOURNAL_FILE)
}

fn lock_journal(path: &Path) -> Result<std::fs::File, FleetSubnetRootInstallJournalError> {
    let lock_path = path.with_file_name(JOURNAL_LOCK_FILE);
    match lock_regular_file_with_parents(&lock_path) {
        Ok(lock) => Ok(lock),
        Err(RegularFileLockError::NotRegular) => {
            Err(FleetSubnetRootInstallJournalError::UnsafeLock { path: lock_path })
        }
        Err(RegularFileLockError::Io(source)) => Err(FleetSubnetRootInstallJournalError::Io {
            path: lock_path,
            source,
        }),
        #[cfg(windows)]
        Err(RegularFileLockError::UnsupportedPlatform) => {
            Err(FleetSubnetRootInstallJournalError::Io {
                path: lock_path,
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Fleet Subnet Root install journal locking is unsupported",
                ),
            })
        }
    }
}

const fn resolved(
    journal: FleetSubnetRootInstallJournal,
    path: PathBuf,
    advanced: bool,
) -> ResolvedFleetSubnetRootInstall {
    ResolvedFleetSubnetRootInstall {
        journal,
        path,
        advanced,
    }
}

fn invalid(path: &Path, reason: impl Into<String>) -> FleetSubnetRootInstallJournalError {
    FleetSubnetRootInstallJournalError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}
