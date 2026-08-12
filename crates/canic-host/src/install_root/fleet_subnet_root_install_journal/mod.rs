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
        BoundedRegularFileReadError, CanonicalJsonEncodeError, CanonicalJsonStyle,
        ExactReplaceError, RegularFileLockError, RegularFileReadError,
        create_new_bytes_with_parents, encode_canonical_json, lock_regular_file_with_parents,
        read_optional_bounded_regular_bytes, replace_bytes_exact,
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
        component_registry::{
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
        },
        fleet_registry::{
            FleetDirectorySnapshot, FleetRegistry, FleetRegistryManifest, FleetRegistryVersion,
            FleetSubnetRootEntry, FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse,
            FleetSubnetRootRegistryMirrorActivationRequest,
            FleetSubnetRootRegistryMirrorActivationResponse, FleetSubnetRootRegistrySyncRequest,
            FleetSubnetRootRegistrySyncResponse, FleetSubnetRootSnapshotAcknowledgement,
            FleetSubnetRootStatus,
        },
        fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetWasmStoreAdoptionResponse},
        root_store::RootStoreBootstrapResponse,
    },
    ids::{
        FleetCoordinatorBinding, FleetRegistryAuthority, FleetSubnetRootBinding,
        FleetSubnetWasmStoreAuthority, ReleaseBuildId, SubnetId,
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
const ROOT_CREATE_RESULT_FILE: &str = "root-create-result.json";
const WASM_STORE_CREATE_RESULT_FILE: &str = "wasm-store-create-result.json";
const ROOT_INSTALL_DIRECTORY: &str = "fleet-subnet-root-installs";
const JOURNAL_SCHEMA_VERSION: u32 = 1;
const MAX_JOURNAL_BYTES: usize = 4_194_304;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum FleetSubnetRootInstallPhase {
    Planned,
    RootCreationInFlight,
    RootCreated,
    WasmStoreCreationInFlight,
    WasmStoreCreated,
    WasmStoreInstallInFlight,
    WasmStoreInstalled,
    RootInstallInFlight,
    RootInstalled,
    InfrastructureVerified,
    StoreAdoptionInFlight,
    StoreAdopted,
    StoreStaging,
    StoreStaged,
    StoreBootstrapInFlight,
    StoreBootstrapped,
    StoreVerified,
    RegistryJoinInFlight,
    RegistryJoined,
    RegistryJoinVerified,
    RegistrySyncInFlight,
    RegistrySynchronized,
    RegistrySyncVerified,
    RegistryMirrorActivationInFlight,
    RegistryMirrorActivated,
    RegistryMirrorActivationVerified,
    ComponentRegistryPreparationInFlight,
    ComponentRegistryPrepared,
    ComponentRegistryPreparationVerified,
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
    pub expected_root_module_hash: [u8; 32],
    pub wasm_store_artifact: CanicInfrastructureArtifactEntry,
    pub expected_wasm_store_module_hash: [u8; 32],
    pub fleet_subnet_root: Option<Principal>,
    pub wasm_store: Option<Principal>,
    pub installation_controller: Option<Principal>,
    pub installed_wasm_store_module_hash: Option<[u8; 32]>,
    pub installed_root_module_hash: Option<[u8; 32]>,
    pub verified_wasm_store_authority: Option<FleetSubnetWasmStoreAuthority>,
    pub verified_binding: Option<FleetSubnetRootBinding>,
    pub wasm_store_adoption: Option<FleetSubnetWasmStoreAdoptionResponse>,
    pub store_bootstrap: Option<RootStoreBootstrapResponse>,
    pub registry_join_request: Option<FleetSubnetRootJoinRequest>,
    pub registry_join_response: Option<FleetSubnetRootJoinResponse>,
    pub registry_sync_request: Option<FleetSubnetRootRegistrySyncRequest>,
    pub registry_sync_response: Option<FleetSubnetRootRegistrySyncResponse>,
    pub registry_mirror_activation_request: Option<FleetSubnetRootRegistryMirrorActivationRequest>,
    pub registry_mirror_activation_response:
        Option<FleetSubnetRootRegistryMirrorActivationResponse>,
    pub component_registry_preparation_request: Option<RootComponentRegistryPreparationRequest>,
    pub component_registry_preparation_response: Option<RootComponentRegistryStatusResponse>,
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

#[derive(Eq, PartialEq)]
struct FleetSubnetRootInstallAuthority<'a> {
    schema_version: u32,
    fleet_install_plan_digest: &'a [u8; 32],
    infrastructure_manifest_digest: &'a [u8; 32],
    authority: &'a FleetRegistryAuthority,
    release_build_id: &'a ReleaseBuildId,
    install_operation_id: &'a [u8; 32],
    component_topology: &'a ComponentTopology,
    root_plan: &'a PlannedFleetSubnetRoot,
    root_artifact: &'a CanicInfrastructureArtifactEntry,
    expected_root_module_hash: &'a [u8; 32],
    wasm_store_artifact: &'a CanicInfrastructureArtifactEntry,
    expected_wasm_store_module_hash: &'a [u8; 32],
}

#[derive(Debug, ThisError)]
pub(super) enum FleetSubnetRootInstallJournalError {
    #[error("Fleet Subnet Root install journal already has different authority: {path}")]
    ConflictingAuthority { path: PathBuf },

    #[error("Fleet Subnet Root infrastructure artifact entry is missing")]
    RootArtifactMissing,

    #[error("Fleet Subnet Root artifact SHA-256 is not one 32-byte digest")]
    InvalidRootArtifactDigest,

    #[error("Wasm Store infrastructure artifact entry is missing")]
    WasmStoreArtifactMissing,

    #[error("Wasm Store artifact SHA-256 is not one 32-byte digest")]
    InvalidWasmStoreArtifactDigest,

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
    installation_controller: Principal,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    if current.journal.phase == FleetSubnetRootInstallPhase::RootCreationInFlight
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
        FleetSubnetRootInstallPhase::Planned,
        FleetSubnetRootInstallPhase::RootCreationInFlight,
        |next| next.installation_controller = Some(installation_controller),
    )
}

pub(super) fn record_root_created(
    current: &ResolvedFleetSubnetRootInstall,
    fleet_subnet_root: Principal,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    if current.journal.phase == FleetSubnetRootInstallPhase::RootCreated
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
        FleetSubnetRootInstallPhase::RootCreationInFlight,
        FleetSubnetRootInstallPhase::RootCreated,
        |next| next.fleet_subnet_root = Some(fleet_subnet_root),
    )
}

pub(super) fn begin_wasm_store_creation(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::RootCreated,
        FleetSubnetRootInstallPhase::WasmStoreCreationInFlight,
    )
}

pub(super) fn record_wasm_store_created(
    current: &ResolvedFleetSubnetRootInstall,
    wasm_store: Principal,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    if current.journal.phase == FleetSubnetRootInstallPhase::WasmStoreCreated
        && current.journal.wasm_store == Some(wasm_store)
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetSubnetRootInstallPhase::WasmStoreCreationInFlight,
        FleetSubnetRootInstallPhase::WasmStoreCreated,
        |next| next.wasm_store = Some(wasm_store),
    )
}

pub(super) fn begin_wasm_store_install(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::WasmStoreCreated,
        FleetSubnetRootInstallPhase::WasmStoreInstallInFlight,
    )
}

pub(super) fn record_wasm_store_installed(
    current: &ResolvedFleetSubnetRootInstall,
    observed_module_hash: [u8; 32],
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    if observed_module_hash != current.journal.expected_wasm_store_module_hash {
        return Err(invalid(
            &current.path,
            "observed Wasm Store module differs from immutable artifact authority",
        ));
    }
    if current.journal.phase == FleetSubnetRootInstallPhase::WasmStoreInstalled
        && current.journal.installed_wasm_store_module_hash == Some(observed_module_hash)
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetSubnetRootInstallPhase::WasmStoreInstallInFlight,
        FleetSubnetRootInstallPhase::WasmStoreInstalled,
        |next| next.installed_wasm_store_module_hash = Some(observed_module_hash),
    )
}

pub(super) fn begin_root_install(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::WasmStoreInstalled,
        FleetSubnetRootInstallPhase::RootInstallInFlight,
    )
}

pub(super) fn record_root_installed(
    current: &ResolvedFleetSubnetRootInstall,
    observed_module_hash: [u8; 32],
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    if observed_module_hash != current.journal.expected_root_module_hash {
        return Err(invalid(
            &current.path,
            "observed root module differs from immutable artifact authority",
        ));
    }
    if current.journal.phase == FleetSubnetRootInstallPhase::RootInstalled
        && current.journal.installed_root_module_hash == Some(observed_module_hash)
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetSubnetRootInstallPhase::RootInstallInFlight,
        FleetSubnetRootInstallPhase::RootInstalled,
        |next| next.installed_root_module_hash = Some(observed_module_hash),
    )
}

pub(super) fn record_infrastructure_verified(
    current: &ResolvedFleetSubnetRootInstall,
    root_authority: FleetSubnetRootAuthority,
    wasm_store_authority: FleetSubnetWasmStoreAuthority,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    let expected_root = expected_root_authority(&current.journal)?;
    let expected_store = expected_wasm_store_authority(&current.journal)?;
    if root_authority != expected_root || wasm_store_authority != expected_store {
        return Err(invalid(
            &current.path,
            "observed infrastructure authority differs from the exact planned binding",
        ));
    }
    let evidence_matches = current.journal.verified_binding.as_ref()
        == Some(&root_authority.binding)
        && current.journal.verified_wasm_store_authority.as_ref() == Some(&wasm_store_authority);
    if current.journal.phase == FleetSubnetRootInstallPhase::InfrastructureVerified
        && evidence_matches
    {
        return Ok(resolved(
            current.journal.clone(),
            current.path.clone(),
            false,
        ));
    }
    transition(
        current,
        FleetSubnetRootInstallPhase::RootInstalled,
        FleetSubnetRootInstallPhase::InfrastructureVerified,
        |next| {
            next.verified_binding = Some(root_authority.binding);
            next.verified_wasm_store_authority = Some(wasm_store_authority);
        },
    )
}

pub(super) fn begin_store_staging(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::StoreAdopted,
        FleetSubnetRootInstallPhase::StoreStaging,
    )
}

pub(super) fn begin_store_adoption(
    current: &ResolvedFleetSubnetRootInstall,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::InfrastructureVerified,
        FleetSubnetRootInstallPhase::StoreAdoptionInFlight,
    )
}

pub(super) fn record_store_adopted(
    current: &ResolvedFleetSubnetRootInstall,
    evidence: FleetSubnetWasmStoreAdoptionResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_wasm_store_adoption_evidence(&current.path, &current.journal, &evidence)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::StoreAdoptionInFlight,
        FleetSubnetRootInstallPhase::StoreAdopted,
        |next| next.wasm_store_adoption = Some(evidence),
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

pub(super) fn begin_registry_sync(
    current: &ResolvedFleetSubnetRootInstall,
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_registry_sync_request(&current.path, &current.journal, &request)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::RegistryJoinVerified,
        FleetSubnetRootInstallPhase::RegistrySyncInFlight,
        |next| next.registry_sync_request = Some(request),
    )
}

pub(super) fn record_registry_synchronized(
    current: &ResolvedFleetSubnetRootInstall,
    response: FleetSubnetRootRegistrySyncResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_registry_sync_response(&current.path, &current.journal, &response)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::RegistrySyncInFlight,
        FleetSubnetRootInstallPhase::RegistrySynchronized,
        |next| next.registry_sync_response = Some(response),
    )
}

pub(super) fn record_registry_sync_verified(
    current: &ResolvedFleetSubnetRootInstall,
    response: FleetSubnetRootRegistrySyncResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_registry_sync_response(&current.path, &current.journal, &response)?;
    if current.journal.registry_sync_response.as_ref() != Some(&response) {
        return Err(invalid(
            &current.path,
            "verified Registry synchronization differs from its durable result",
        ));
    }
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::RegistrySynchronized,
        FleetSubnetRootInstallPhase::RegistrySyncVerified,
    )
}

pub(super) fn begin_registry_mirror_activation(
    current: &ResolvedFleetSubnetRootInstall,
    request: FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_registry_mirror_activation_request(&current.path, &current.journal, &request)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::RegistrySyncVerified,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight,
        |next| next.registry_mirror_activation_request = Some(request),
    )
}

pub(super) fn record_registry_mirror_activated(
    current: &ResolvedFleetSubnetRootInstall,
    response: FleetSubnetRootRegistryMirrorActivationResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_registry_mirror_activation_response(&current.path, &current.journal, &response)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight,
        FleetSubnetRootInstallPhase::RegistryMirrorActivated,
        |next| next.registry_mirror_activation_response = Some(response),
    )
}

pub(super) fn record_registry_mirror_activation_verified(
    current: &ResolvedFleetSubnetRootInstall,
    response: FleetSubnetRootRegistryMirrorActivationResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_registry_mirror_activation_response(&current.path, &current.journal, &response)?;
    if current.journal.registry_mirror_activation_response.as_ref() != Some(&response) {
        return Err(invalid(
            &current.path,
            "verified Registry mirror activation differs from its durable result",
        ));
    }
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::RegistryMirrorActivated,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified,
    )
}

pub(super) fn begin_component_registry_preparation(
    current: &ResolvedFleetSubnetRootInstall,
    request: RootComponentRegistryPreparationRequest,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_component_registry_preparation_request(&current.path, &current.journal, &request)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight,
        |next| next.component_registry_preparation_request = Some(request),
    )
}

pub(super) fn record_component_registry_prepared(
    current: &ResolvedFleetSubnetRootInstall,
    response: RootComponentRegistryStatusResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_component_registry_preparation_response(&current.path, &current.journal, &response)?;
    transition(
        current,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight,
        FleetSubnetRootInstallPhase::ComponentRegistryPrepared,
        |next| next.component_registry_preparation_response = Some(response),
    )
}

pub(super) fn record_component_registry_preparation_verified(
    current: &ResolvedFleetSubnetRootInstall,
    response: RootComponentRegistryStatusResponse,
) -> Result<ResolvedFleetSubnetRootInstall, FleetSubnetRootInstallJournalError> {
    validate_component_registry_preparation_response(&current.path, &current.journal, &response)?;
    if current
        .journal
        .component_registry_preparation_response
        .as_ref()
        != Some(&response)
    {
        return Err(invalid(
            &current.path,
            "verified Component Registry preparation differs from its durable result",
        ));
    }
    advance_without_evidence(
        current,
        FleetSubnetRootInstallPhase::ComponentRegistryPrepared,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified,
    )
}

#[must_use]
pub(super) fn create_result_path(journal_path: &Path) -> PathBuf {
    journal_path
        .parent()
        .expect("validated root journal has an identity directory")
        .join(ROOT_CREATE_RESULT_FILE)
}

#[must_use]
pub(super) fn wasm_store_create_result_path(journal_path: &Path) -> PathBuf {
    journal_path
        .parent()
        .expect("validated root journal has an identity directory")
        .join(WASM_STORE_CREATE_RESULT_FILE)
}

pub(super) fn expected_wasm_store_authority(
    journal: &FleetSubnetRootInstallJournal,
) -> Result<FleetSubnetWasmStoreAuthority, FleetSubnetRootInstallJournalError> {
    let fleet_subnet_root = journal
        .fleet_subnet_root
        .ok_or_else(|| invalid(Path::new(JOURNAL_FILE), "root principal is missing"))?;
    let wasm_store = journal
        .wasm_store
        .ok_or_else(|| invalid(Path::new(JOURNAL_FILE), "Wasm Store principal is missing"))?;
    let installation_controller = journal.installation_controller.ok_or_else(|| {
        invalid(
            Path::new(JOURNAL_FILE),
            "installation controller is missing",
        )
    })?;
    Ok(FleetSubnetWasmStoreAuthority {
        authority: journal.authority.clone(),
        placement_subnet: journal.root_plan.placement_subnet,
        fleet_subnet_root,
        wasm_store,
        installation_controller,
        release_build_id: journal.release_build_id,
        wasm_module_hash: journal.expected_wasm_store_module_hash,
    })
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
        expected_module_hash: journal.expected_root_module_hash,
        wasm_store_authority: expected_wasm_store_authority(journal)?,
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
    let expected_root_module_hash = decode_hex(&root_artifact.wasm_sha256_hex)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or(FleetSubnetRootInstallJournalError::InvalidRootArtifactDigest)?;
    let wasm_store_artifact = request
        .infrastructure_manifest
        .manifest
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::WasmStore)
        .cloned()
        .ok_or(FleetSubnetRootInstallJournalError::WasmStoreArtifactMissing)?;
    let expected_wasm_store_module_hash = decode_hex(&wasm_store_artifact.wasm_sha256_hex)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or(FleetSubnetRootInstallJournalError::InvalidWasmStoreArtifactDigest)?;
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
        expected_root_module_hash,
        wasm_store_artifact,
        expected_wasm_store_module_hash,
        fleet_subnet_root: None,
        wasm_store: None,
        installation_controller: None,
        installed_wasm_store_module_hash: None,
        installed_root_module_hash: None,
        verified_wasm_store_authority: None,
        verified_binding: None,
        wasm_store_adoption: None,
        store_bootstrap: None,
        registry_join_request: None,
        registry_join_response: None,
        registry_sync_request: None,
        registry_sync_response: None,
        registry_mirror_activation_request: None,
        registry_mirror_activation_response: None,
        component_registry_preparation_request: None,
        component_registry_preparation_response: None,
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
    replace_bytes_exact(&current.path, &bytes)
        .map_err(|error| replace_error(&current.path, error))?;
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
    let bytes = match read_optional_bounded_regular_bytes(path, MAX_JOURNAL_BYTES) {
        Ok(bytes) => bytes,
        Err(BoundedRegularFileReadError::TooLarge) => {
            return Err(invalid(path, "journal exceeds its byte bound"));
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            return Err(FleetSubnetRootInstallJournalError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            return Err(FleetSubnetRootInstallJournalError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
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
    if journal
        .installation_controller
        .is_some_and(|controller| controller == Principal::anonymous())
    {
        return Err(invalid(path, "installation controller is anonymous"));
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
    if journal.expected_root_module_hash != artifact_hash {
        return Err(invalid(
            path,
            "expected module hash differs from root artifact",
        ));
    }
    if journal.wasm_store_artifact.role != CanicInfrastructureRole::WasmStore
        || journal.wasm_store_artifact.release_build_id != journal.release_build_id
    {
        return Err(invalid(
            path,
            "Wasm Store artifact differs from role or release build authority",
        ));
    }
    let wasm_store_artifact_hash = decode_hex(&journal.wasm_store_artifact.wasm_sha256_hex)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| invalid(path, "Wasm Store artifact digest is invalid"))?;
    if journal.expected_wasm_store_module_hash != wasm_store_artifact_hash {
        return Err(invalid(
            path,
            "expected module hash differs from Wasm Store artifact",
        ));
    }
    Ok(())
}

fn validate_phase_evidence(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    if journal.sequence != phase_sequence(journal.phase) {
        return Err(invalid(path, "phase differs from journal sequence"));
    }
    let retained = [
        journal.installation_controller.is_some(),
        journal.fleet_subnet_root.is_some(),
        journal.wasm_store.is_some(),
        journal.installed_wasm_store_module_hash.is_some(),
        journal.installed_root_module_hash.is_some(),
        journal.verified_wasm_store_authority.is_some(),
        journal.verified_binding.is_some(),
        journal.wasm_store_adoption.is_some(),
        journal.store_bootstrap.is_some(),
        journal.registry_join_request.is_some(),
        journal.registry_join_response.is_some(),
        journal.registry_sync_request.is_some(),
        journal.registry_sync_response.is_some(),
        journal.registry_mirror_activation_request.is_some(),
        journal.registry_mirror_activation_response.is_some(),
        journal.component_registry_preparation_request.is_some(),
        journal.component_registry_preparation_response.is_some(),
    ];
    let expected_count = phase_evidence_count(journal.phase);
    if retained
        .into_iter()
        .enumerate()
        .any(|(index, present)| present != (index < expected_count))
    {
        return Err(invalid(path, "phase differs from retained root evidence"));
    }
    if journal.installed_root_module_hash.is_some()
        && journal.installed_root_module_hash != Some(journal.expected_root_module_hash)
    {
        return Err(invalid(
            path,
            "installed module differs from artifact authority",
        ));
    }
    if journal.installed_wasm_store_module_hash.is_some()
        && journal.installed_wasm_store_module_hash != Some(journal.expected_wasm_store_module_hash)
    {
        return Err(invalid(
            path,
            "installed Wasm Store module differs from artifact authority",
        ));
    }
    Ok(())
}

const fn phase_sequence(phase: FleetSubnetRootInstallPhase) -> u64 {
    match phase {
        FleetSubnetRootInstallPhase::Planned => 0,
        FleetSubnetRootInstallPhase::RootCreationInFlight => 1,
        FleetSubnetRootInstallPhase::RootCreated => 2,
        FleetSubnetRootInstallPhase::WasmStoreCreationInFlight => 3,
        FleetSubnetRootInstallPhase::WasmStoreCreated => 4,
        FleetSubnetRootInstallPhase::WasmStoreInstallInFlight => 5,
        FleetSubnetRootInstallPhase::WasmStoreInstalled => 6,
        FleetSubnetRootInstallPhase::RootInstallInFlight => 7,
        FleetSubnetRootInstallPhase::RootInstalled => 8,
        FleetSubnetRootInstallPhase::InfrastructureVerified => 9,
        FleetSubnetRootInstallPhase::StoreAdoptionInFlight => 10,
        FleetSubnetRootInstallPhase::StoreAdopted => 11,
        FleetSubnetRootInstallPhase::StoreStaging => 12,
        FleetSubnetRootInstallPhase::StoreStaged => 13,
        FleetSubnetRootInstallPhase::StoreBootstrapInFlight => 14,
        FleetSubnetRootInstallPhase::StoreBootstrapped => 15,
        FleetSubnetRootInstallPhase::StoreVerified => 16,
        FleetSubnetRootInstallPhase::RegistryJoinInFlight => 17,
        FleetSubnetRootInstallPhase::RegistryJoined => 18,
        FleetSubnetRootInstallPhase::RegistryJoinVerified => 19,
        FleetSubnetRootInstallPhase::RegistrySyncInFlight => 20,
        FleetSubnetRootInstallPhase::RegistrySynchronized => 21,
        FleetSubnetRootInstallPhase::RegistrySyncVerified => 22,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight => 23,
        FleetSubnetRootInstallPhase::RegistryMirrorActivated => 24,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified => 25,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight => 26,
        FleetSubnetRootInstallPhase::ComponentRegistryPrepared => 27,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => 28,
    }
}

const fn phase_evidence_count(phase: FleetSubnetRootInstallPhase) -> usize {
    match phase {
        FleetSubnetRootInstallPhase::Planned => 0,
        FleetSubnetRootInstallPhase::RootCreationInFlight => 1,
        FleetSubnetRootInstallPhase::RootCreated
        | FleetSubnetRootInstallPhase::WasmStoreCreationInFlight => 2,
        FleetSubnetRootInstallPhase::WasmStoreCreated
        | FleetSubnetRootInstallPhase::WasmStoreInstallInFlight => 3,
        FleetSubnetRootInstallPhase::WasmStoreInstalled
        | FleetSubnetRootInstallPhase::RootInstallInFlight => 4,
        FleetSubnetRootInstallPhase::RootInstalled => 5,
        FleetSubnetRootInstallPhase::InfrastructureVerified
        | FleetSubnetRootInstallPhase::StoreAdoptionInFlight => 7,
        FleetSubnetRootInstallPhase::StoreAdopted
        | FleetSubnetRootInstallPhase::StoreStaging
        | FleetSubnetRootInstallPhase::StoreStaged
        | FleetSubnetRootInstallPhase::StoreBootstrapInFlight => 8,
        FleetSubnetRootInstallPhase::StoreBootstrapped
        | FleetSubnetRootInstallPhase::StoreVerified => 9,
        FleetSubnetRootInstallPhase::RegistryJoinInFlight => 10,
        FleetSubnetRootInstallPhase::RegistryJoined
        | FleetSubnetRootInstallPhase::RegistryJoinVerified => 11,
        FleetSubnetRootInstallPhase::RegistrySyncInFlight => 12,
        FleetSubnetRootInstallPhase::RegistrySynchronized
        | FleetSubnetRootInstallPhase::RegistrySyncVerified => 13,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight => 14,
        FleetSubnetRootInstallPhase::RegistryMirrorActivated
        | FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified => 15,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight => 16,
        FleetSubnetRootInstallPhase::ComponentRegistryPrepared
        | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified => 17,
    }
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
    let Some(installation_controller) = journal.installation_controller else {
        return Ok(());
    };
    let invalid_controllers = [Principal::anonymous(), fleet_subnet_root];
    if invalid_controllers.contains(&installation_controller) {
        return Err(invalid(
            path,
            "installation controller conflicts with sibling infrastructure",
        ));
    }
    let Some(wasm_store) = journal.wasm_store else {
        return Ok(());
    };
    let protected_canisters = [
        Principal::anonymous(),
        journal.authority.binding.coordinator,
        fleet_subnet_root,
    ];
    if protected_canisters.contains(&wasm_store) {
        return Err(invalid(
            path,
            "Wasm Store principal conflicts with Fleet authority",
        ));
    }
    if installation_controller == wasm_store {
        return Err(invalid(
            path,
            "installation controller conflicts with sibling infrastructure",
        ));
    }
    let expected =
        expected_root_authority(journal).map_err(|error| invalid(path, error.to_string()))?;
    if matches!(
        journal.phase,
        FleetSubnetRootInstallPhase::InfrastructureVerified
            | FleetSubnetRootInstallPhase::StoreAdoptionInFlight
            | FleetSubnetRootInstallPhase::StoreAdopted
            | FleetSubnetRootInstallPhase::StoreStaging
            | FleetSubnetRootInstallPhase::StoreStaged
            | FleetSubnetRootInstallPhase::StoreBootstrapInFlight
            | FleetSubnetRootInstallPhase::StoreBootstrapped
            | FleetSubnetRootInstallPhase::StoreVerified
            | FleetSubnetRootInstallPhase::RegistryJoinInFlight
            | FleetSubnetRootInstallPhase::RegistryJoined
            | FleetSubnetRootInstallPhase::RegistryJoinVerified
            | FleetSubnetRootInstallPhase::RegistrySyncInFlight
            | FleetSubnetRootInstallPhase::RegistrySynchronized
            | FleetSubnetRootInstallPhase::RegistrySyncVerified
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight
            | FleetSubnetRootInstallPhase::RegistryMirrorActivated
            | FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight
            | FleetSubnetRootInstallPhase::ComponentRegistryPrepared
            | FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified
    ) {
        let expected_store = &expected.wasm_store_authority;
        let root_matches = journal.verified_binding.as_ref() == Some(&expected.binding);
        let store_matches = journal.verified_wasm_store_authority.as_ref() == Some(expected_store);
        if root_matches && store_matches {
            return validate_root_dependent_evidence(path, journal);
        }
        return Err(invalid(
            path,
            "verified infrastructure authority differs from the exact planned binding",
        ));
    }
    validate_root_dependent_evidence(path, journal)
}

fn validate_root_dependent_evidence(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    if let Some(evidence) = &journal.wasm_store_adoption {
        validate_wasm_store_adoption_evidence(path, journal, evidence)?;
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
    if let Some(request) = &journal.registry_sync_request {
        validate_registry_sync_request(path, journal, request)?;
    }
    if let Some(response) = &journal.registry_sync_response {
        validate_registry_sync_response(path, journal, response)?;
    }
    if let Some(request) = &journal.registry_mirror_activation_request {
        validate_registry_mirror_activation_request(path, journal, request)?;
    }
    if let Some(response) = &journal.registry_mirror_activation_response {
        validate_registry_mirror_activation_response(path, journal, response)?;
    }
    if let Some(request) = &journal.component_registry_preparation_request {
        validate_component_registry_preparation_request(path, journal, request)?;
    }
    if let Some(response) = &journal.component_registry_preparation_response {
        validate_component_registry_preparation_response(path, journal, response)?;
    }
    Ok(())
}

fn validate_wasm_store_adoption_evidence(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    evidence: &FleetSubnetWasmStoreAdoptionResponse,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    if evidence.adopted_at_ns == 0 {
        return Err(invalid(path, "Store adoption timestamp must be nonzero"));
    }
    let authority =
        expected_wasm_store_authority(journal).map_err(|error| invalid(path, error.to_string()))?;
    let mut temporary_controllers = vec![
        authority.installation_controller,
        authority.fleet_subnet_root,
    ];
    temporary_controllers.sort();
    let expected = FleetSubnetWasmStoreAdoptionResponse {
        operation_id: journal.install_operation_id,
        authority: authority.clone(),
        temporary_controllers,
        final_controllers: vec![authority.fleet_subnet_root],
        adopted_at_ns: evidence.adopted_at_ns,
    };
    if evidence != &expected {
        return Err(invalid(
            path,
            "Store adoption receipt differs from immutable install authority",
        ));
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
    let protected_principals = [
        Principal::anonymous(),
        fleet_subnet_root,
        journal.authority.binding.coordinator,
    ];
    let root_authority_matches = evidence.fleet_subnet_root == fleet_subnet_root
        && evidence.release_set == journal.root_plan.initial_release_set;
    let store_evidence_is_complete =
        !protected_principals.contains(&evidence.wasm_store) && !evidence.catalog.is_empty();
    if !root_authority_matches || !store_evidence_is_complete {
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
    if response.version.content_hash == [0; 32] {
        return Err(invalid(
            path,
            "Registry join response has no committed content hash",
        ));
    }
    let expected = FleetSubnetRootJoinResponse {
        entry: request.entry.clone(),
        version: FleetRegistryVersion {
            authority: journal.authority.clone(),
            revision: expected_revision,
            content_hash: response.version.content_hash,
        },
    };
    if response != &expected {
        return Err(invalid(
            path,
            "Registry join response differs from the durable request",
        ));
    }
    Ok(())
}

fn validate_registry_sync_request(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    request: &FleetSubnetRootRegistrySyncRequest,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    if request.expected_registry.authority != journal.authority
        || request.expected_registry.revision == 0
        || request.expected_registry.content_hash == [0; 32]
        || request.store_bootstrap.manifest_payload_size_bytes == 0
    {
        return Err(invalid(
            path,
            "Registry synchronization request differs from root authority",
        ));
    }
    Ok(())
}

fn validate_registry_sync_response(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    response: &FleetSubnetRootRegistrySyncResponse,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let request = journal.registry_sync_request.as_ref().ok_or_else(|| {
        invalid(
            path,
            "Registry synchronization response requires a durable request",
        )
    })?;
    let root = journal
        .fleet_subnet_root
        .ok_or_else(|| invalid(path, "Registry synchronization requires a root principal"))?;
    let expected = FleetSubnetRootRegistrySyncResponse {
        fleet_subnet_root: root,
        version: request.expected_registry.clone(),
        acknowledgement: FleetSubnetRootSnapshotAcknowledgement {
            fleet_subnet_root: root,
            version: request.expected_registry.clone(),
        },
    };
    if response != &expected {
        return Err(invalid(
            path,
            "Registry synchronization response differs from durable authority",
        ));
    }
    Ok(())
}

fn validate_registry_mirror_activation_request(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    request: &FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let sync_request = journal.registry_sync_request.as_ref().ok_or_else(|| {
        invalid(
            path,
            "Registry mirror activation requires a durable synchronization request",
        )
    })?;
    let root = journal
        .fleet_subnet_root
        .ok_or_else(|| invalid(path, "Registry mirror activation requires a root principal"))?;
    let registry_authority_advances = request.previous_registry == sync_request.expected_registry
        && request.expected_registry.authority == journal.authority
        && request.previous_registry.revision.checked_add(1)
            == Some(request.expected_registry.revision)
        && request.expected_registry.content_hash != [0; 32];
    let store_authority_matches = request.store_bootstrap == sync_request.store_bootstrap;
    let directory_authority_matches = request.expected_directory.provenance.registry
        == request.expected_registry
        && request
            .expected_directory
            .provenance
            .source_fleet_subnet_root
            == root;
    if !registry_authority_advances || !store_authority_matches || !directory_authority_matches {
        return Err(invalid(
            path,
            "Registry mirror activation request differs from durable root authority",
        ));
    }
    validate_fleet_directory(path, &request.expected_directory)?;
    Ok(())
}

fn validate_registry_mirror_activation_response(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    response: &FleetSubnetRootRegistryMirrorActivationResponse,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let request = journal
        .registry_mirror_activation_request
        .as_ref()
        .ok_or_else(|| {
            invalid(
                path,
                "Registry mirror activation response requires a durable request",
            )
        })?;
    let root = journal
        .fleet_subnet_root
        .ok_or_else(|| invalid(path, "Registry mirror activation requires a root principal"))?;
    let expected = FleetSubnetRootRegistryMirrorActivationResponse {
        fleet_subnet_root: root,
        previous_registry: request.previous_registry.clone(),
        version: request.expected_registry.clone(),
        directory: request.expected_directory.clone(),
    };
    if response != &expected {
        return Err(invalid(
            path,
            "Registry mirror activation response differs from durable authority",
        ));
    }
    Ok(())
}

fn validate_component_registry_preparation_request(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    request: &RootComponentRegistryPreparationRequest,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let mirror_request = journal
        .registry_mirror_activation_request
        .as_ref()
        .ok_or_else(|| {
            invalid(
                path,
                "Component Registry preparation requires a durable mirror activation request",
            )
        })?;
    let store_matches = request.store_bootstrap == mirror_request.store_bootstrap;
    let registry_matches = request.expected_fleet_registry == mirror_request.expected_registry;
    if !store_matches || !registry_matches {
        return Err(invalid(
            path,
            "Component Registry preparation request differs from durable root authority",
        ));
    }
    Ok(())
}

fn validate_component_registry_preparation_response(
    path: &Path,
    journal: &FleetSubnetRootInstallJournal,
    response: &RootComponentRegistryStatusResponse,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    let request = journal
        .component_registry_preparation_request
        .as_ref()
        .ok_or_else(|| {
            invalid(
                path,
                "Component Registry preparation response requires a durable request",
            )
        })?;
    let root = journal.fleet_subnet_root.ok_or_else(|| {
        invalid(
            path,
            "Component Registry preparation requires a root principal",
        )
    })?;
    let expected = RootComponentRegistryStatusResponse {
        fleet_subnet_root: root,
        prepared_against_registry: request.expected_fleet_registry.clone(),
        release_set: journal.root_plan.initial_release_set,
        component_topology_digest: journal.root_plan.component_topology_digest,
        next_allocation_sequence: 1,
        reserved_component_instances: 0,
        committed_component_instances: 0,
        managed_descendants: 0,
        known_created_component_canisters: 0,
        encoded_bytes: 0,
        initial_inventory: None,
    };
    if response != &expected {
        return Err(invalid(
            path,
            "Component Registry preparation response differs from immutable root authority",
        ));
    }
    Ok(())
}

fn validate_fleet_directory(
    path: &Path,
    directory: &FleetDirectorySnapshot,
) -> Result<(), FleetSubnetRootInstallJournalError> {
    if directory.fleet_subnet_roots.is_empty()
        || !directory
            .fleet_subnet_roots
            .iter()
            .any(|entry| entry.fleet_subnet_root == directory.provenance.source_fleet_subnet_root)
    {
        return Err(invalid(
            path,
            "Fleet Directory does not contain its exact source root",
        ));
    }
    let mut subnets = BTreeSet::new();
    let mut roots = BTreeSet::new();
    let mut previous = None;
    for entry in &directory.fleet_subnet_roots {
        let identity_is_active = entry.placement_subnet.as_principal() != &Principal::anonymous()
            && entry.fleet_subnet_root != Principal::anonymous()
            && entry.status == FleetSubnetRootStatus::Active;
        let identity_is_unique =
            subnets.insert(entry.placement_subnet) && roots.insert(entry.fleet_subnet_root);
        let order_is_canonical = previous.is_none_or(|subnet| subnet < entry.placement_subnet);
        if !identity_is_active || !identity_is_unique || !order_is_canonical {
            return Err(invalid(
                path,
                "Fleet Directory root order or identity is invalid",
            ));
        }
        previous = Some(entry.placement_subnet);
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
    let snapshot_is_canonical = manifest == &expected_manifest && version == &expected_version;
    let receipt_matches = version == &response.version
        && registry
            .fleet_subnet_roots
            .iter()
            .any(|entry| entry == &response.entry);
    if !snapshot_is_canonical || !receipt_matches {
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
    encode_canonical_json(journal, CanonicalJsonStyle::Compact, MAX_JOURNAL_BYTES).map_err(
        |error| match error {
            CanonicalJsonEncodeError::Serialization(error) => invalid(path, error.to_string()),
            CanonicalJsonEncodeError::TooLarge => invalid(path, "journal exceeds its byte bound"),
        },
    )
}

fn replace_error(path: &Path, error: ExactReplaceError) -> FleetSubnetRootInstallJournalError {
    match error {
        ExactReplaceError::Write(source)
        | ExactReplaceError::Read(RegularFileReadError::Io(source)) => {
            FleetSubnetRootInstallJournalError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
        ExactReplaceError::Read(RegularFileReadError::NotRegular) => {
            FleetSubnetRootInstallJournalError::UnsafeFile {
                path: path.to_path_buf(),
            }
        }
        #[cfg(not(unix))]
        ExactReplaceError::Read(RegularFileReadError::UnsupportedPlatform) => {
            FleetSubnetRootInstallJournalError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Fleet Subnet Root install journal reads are unsupported",
                ),
            }
        }
    }
}

fn same_immutable_authority(
    observed: &FleetSubnetRootInstallJournal,
    expected: &FleetSubnetRootInstallJournal,
) -> bool {
    immutable_authority(observed) == immutable_authority(expected)
}

const fn immutable_authority(
    journal: &FleetSubnetRootInstallJournal,
) -> FleetSubnetRootInstallAuthority<'_> {
    FleetSubnetRootInstallAuthority {
        schema_version: journal.schema_version,
        fleet_install_plan_digest: &journal.fleet_install_plan_digest,
        infrastructure_manifest_digest: &journal.infrastructure_manifest_digest,
        authority: &journal.authority,
        release_build_id: &journal.release_build_id,
        install_operation_id: &journal.install_operation_id,
        component_topology: &journal.component_topology,
        root_plan: &journal.root_plan,
        root_artifact: &journal.root_artifact,
        expected_root_module_hash: &journal.expected_root_module_hash,
        wasm_store_artifact: &journal.wasm_store_artifact,
        expected_wasm_store_module_hash: &journal.expected_wasm_store_module_hash,
    }
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
