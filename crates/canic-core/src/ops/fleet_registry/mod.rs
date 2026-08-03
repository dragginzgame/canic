//! Module: ops::fleet_registry
//!
//! Responsibility: compile and encode canonical Fleet Registry snapshot evidence.
//! Does not own: stable commits, lifecycle transitions, synchronization, or endpoints.
//! Boundary: validates passive snapshots against one exact compiled Component Topology.

#[cfg(test)]
mod tests;
mod validation;

use crate::{
    InternalError,
    config::{ComponentTopology, ComponentTopologyError},
    dto::fleet_registry::{
        FleetComponentSpecEntry, FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistry,
        FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootDirectoryEntry,
        FleetSubnetRootEntry, FleetSubnetRootStatus,
    },
    ids::{
        AppId, ComponentSpecAdmission, ComponentSpecId, FleetRegistryAuthority,
        FleetSubnetRootLimits, ReleaseBuildId,
    },
    ops::OpsError,
};
use candid::Principal;
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

const FLEET_REGISTRY_DOMAIN: &[u8] = b"canic/fleet-registry/v1";
const FLEET_REGISTRY_SCHEMA_VERSION: u32 = 1;

/// Maximum canonical bytes accepted for one Fleet Registry snapshot.
pub const MAX_FLEET_REGISTRY_CANONICAL_BYTES: usize = 2_097_152;

///
/// FleetRegistryOpsError
///
/// Typed operations-layer failure while validating or compiling canonical Registry evidence.
///

#[derive(Debug, ThisError)]
pub enum FleetRegistryOpsError {
    #[error("Fleet Registry Coordinator principal must not be anonymous")]
    AnonymousCoordinator,

    #[error("Fleet Registry Coordinator Subnet must not be anonymous")]
    AnonymousCoordinatorSubnet,

    #[error("Fleet Registry root principal must not be anonymous")]
    AnonymousFleetSubnetRoot,

    #[error("Fleet Registry authority does not match the protected expected authority")]
    AuthorityMismatch,

    #[error("Fleet Registry canonical bytes exceed bound {maximum_bytes}: {actual_bytes}")]
    CanonicalBytesExceeded {
        actual_bytes: usize,
        maximum_bytes: usize,
    },

    #[error("Fleet Registry contains duplicate root principal {fleet_subnet_root}")]
    DuplicateFleetSubnetRoot { fleet_subnet_root: Principal },

    #[error(
        "Fleet Registry admissions for Component Spec '{component_spec}' exceed its Fleet maximum {maximum_fleet_instances}: {admitted}"
    )]
    FleetAdmissionsExceedMaximum {
        component_spec: ComponentSpecId,
        admitted: u32,
        maximum_fleet_instances: u32,
    },

    #[error("Fleet Registry admission total overflowed for Component Spec '{component_spec}'")]
    FleetAdmissionsOverflow { component_spec: ComponentSpecId },

    #[error(
        "Fleet Registry Component Spec '{component_spec}' does not match the compiled topology"
    )]
    FleetComponentSpecMismatch { component_spec: ComponentSpecId },

    #[error("Fleet Registry Component Specs are not the complete compiled topology")]
    FleetComponentSpecSetMismatch,

    #[error("Fleet Registry root join conflicts with an existing Subnet or root principal")]
    FleetSubnetRootJoinIdentityConflict,

    #[error("Fleet Registry root join requires status Joining")]
    FleetSubnetRootJoinRequiresJoining,

    #[error("Fleet Registry activation requires a non-empty all-Joining root set")]
    FleetSubnetRootActivationRequiresAllJoining,

    #[error("Fleet Registry root draining requires an Active target")]
    FleetSubnetRootDrainingRequiresActive,

    #[error("Fleet Registry root draining target {fleet_subnet_root} is missing")]
    FleetSubnetRootDrainingTargetMissing { fleet_subnet_root: Principal },

    #[error("Fleet Registry root removal requires a Draining target")]
    FleetSubnetRootRemovalRequiresDraining,

    #[error("Fleet Registry root removal target {fleet_subnet_root} is missing")]
    FleetSubnetRootRemovalTargetMissing { fleet_subnet_root: Principal },

    #[error("Fleet Directory activation requires a non-empty root set with no Joining rows")]
    FleetDirectoryRequiresPublishedRoots,

    #[error("Fleet Directory source does not name one current non-Removed Registry root")]
    FleetDirectorySourceMissing,

    #[error("Fleet Registry genesis App '{received}' does not match configured App '{expected}'")]
    GenesisAppMismatch { expected: AppId, received: AppId },

    #[error("Fleet Registry genesis requires authority epoch 1, got {0}")]
    GenesisAuthorityEpoch(u64),

    #[error("Fleet Registry root order is not strictly ascending by physical Subnet")]
    NonCanonicalFleetSubnetRootOrder,

    #[error("Fleet Registry authority epoch must be positive")]
    NonPositiveAuthorityEpoch,

    #[error("Fleet Registry revision must be positive")]
    NonPositiveRevision,

    #[error("Fleet Registry revision is exhausted")]
    RevisionExhausted,

    #[error("Fleet Registry root principal conflicts with its Coordinator")]
    RootPrincipalConflictsWithCoordinator,

    #[error(
        "Fleet Registry roots carry different active release builds: expected {expected}, got {received}"
    )]
    RootReleaseBuildMismatch {
        expected: ReleaseBuildId,
        received: ReleaseBuildId,
    },

    #[error(transparent)]
    Topology(#[from] ComponentTopologyError),
}

///
/// FleetRegistryOps
///
/// Deterministic canonical Fleet Registry compiler used by Coordinator workflows.
///

pub struct FleetRegistryOps;

impl FleetRegistryOps {
    /// Compile revision-one empty-root authority from one exact Component Topology.
    pub fn compile_genesis(
        configured_app: &AppId,
        authority: FleetRegistryAuthority,
        topology: &ComponentTopology,
    ) -> Result<FleetRegistry, InternalError> {
        validation::compile_genesis(configured_app, authority, topology)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }

    /// Construct the next canonical snapshot with one exact root added as `Joining`.
    pub fn compile_joining(
        expected_authority: &FleetRegistryAuthority,
        topology: &ComponentTopology,
        current: &FleetRegistry,
        entry: FleetSubnetRootEntry,
    ) -> Result<FleetRegistry, InternalError> {
        compile_joining(expected_authority, topology, current, entry)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }

    /// Construct the next canonical snapshot with every current root atomically `Active`.
    pub fn compile_active(
        expected_authority: &FleetRegistryAuthority,
        topology: &ComponentTopology,
        current: &FleetRegistry,
    ) -> Result<FleetRegistry, InternalError> {
        compile_active(expected_authority, topology, current)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }

    /// Construct the next canonical snapshot with one exact active root marked `Draining`.
    pub fn compile_draining(
        expected_authority: &FleetRegistryAuthority,
        topology: &ComponentTopology,
        current: &FleetRegistry,
        fleet_subnet_root: Principal,
    ) -> Result<FleetRegistry, InternalError> {
        compile_draining(expected_authority, topology, current, fleet_subnet_root)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }

    /// Construct the next canonical snapshot with one exact draining root marked `Removed`.
    pub fn compile_removed(
        expected_authority: &FleetRegistryAuthority,
        topology: &ComponentTopology,
        current: &FleetRegistry,
        fleet_subnet_root: Principal,
    ) -> Result<FleetRegistry, InternalError> {
        compile_removed(expected_authority, topology, current, fleet_subnet_root)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }

    /// Validate the complete Registry snapshot against its compiled topology.
    pub fn validate(
        expected_authority: &FleetRegistryAuthority,
        topology: &ComponentTopology,
        registry: &FleetRegistry,
    ) -> Result<(), InternalError> {
        validation::validate(expected_authority, topology, registry)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }

    /// Encode one validated Registry snapshot with the frozen canonical schema.
    pub fn canonical_bytes(
        expected_authority: &FleetRegistryAuthority,
        topology: &ComponentTopology,
        registry: &FleetRegistry,
    ) -> Result<Vec<u8>, InternalError> {
        canonical_bytes(expected_authority, topology, registry)
            .map_err(OpsError::from)
            .map_err(InternalError::from)
    }

    /// Derive the exact manifest for one complete canonical Registry snapshot.
    pub fn manifest(
        expected_authority: &FleetRegistryAuthority,
        topology: &ComponentTopology,
        registry: &FleetRegistry,
    ) -> Result<FleetRegistryManifest, InternalError> {
        let bytes = canonical_bytes(expected_authority, topology, registry)
            .map_err(OpsError::from)
            .map_err(InternalError::from)?;
        Ok(FleetRegistryManifest {
            authority: registry.authority.clone(),
            revision: registry.revision,
            byte_length: bytes.len() as u64,
            content_hash: Sha256::digest(bytes).into(),
        })
    }

    /// Derive the compact version used by mirrors, acknowledgements, and journals.
    pub fn version(
        expected_authority: &FleetRegistryAuthority,
        topology: &ComponentTopology,
        registry: &FleetRegistry,
    ) -> Result<FleetRegistryVersion, InternalError> {
        let manifest = Self::manifest(expected_authority, topology, registry)?;
        Ok(FleetRegistryVersion {
            authority: manifest.authority,
            revision: manifest.revision,
            content_hash: manifest.content_hash,
        })
    }

    /// Derive one current root's exact Fleet Directory from a complete published Registry.
    pub fn directory_for_root(
        expected_authority: &FleetRegistryAuthority,
        topology: &ComponentTopology,
        registry: &FleetRegistry,
        source_fleet_subnet_root: Principal,
    ) -> Result<FleetDirectorySnapshot, InternalError> {
        directory_for_root(
            expected_authority,
            topology,
            registry,
            source_fleet_subnet_root,
        )
        .map_err(OpsError::from)
        .map_err(InternalError::from)
    }
}

fn directory_for_root(
    expected_authority: &FleetRegistryAuthority,
    topology: &ComponentTopology,
    registry: &FleetRegistry,
    source_fleet_subnet_root: Principal,
) -> Result<FleetDirectorySnapshot, FleetRegistryOpsError> {
    validation::validate(expected_authority, topology, registry)?;
    let contains_joining_root = registry
        .fleet_subnet_roots
        .iter()
        .any(|entry| entry.status == FleetSubnetRootStatus::Joining);
    if registry.fleet_subnet_roots.is_empty() || contains_joining_root {
        return Err(FleetRegistryOpsError::FleetDirectoryRequiresPublishedRoots);
    }
    let source_is_current = registry.fleet_subnet_roots.iter().any(|entry| {
        entry.fleet_subnet_root == source_fleet_subnet_root
            && entry.status != FleetSubnetRootStatus::Removed
    });
    if !source_is_current {
        return Err(FleetRegistryOpsError::FleetDirectorySourceMissing);
    }
    let manifest = {
        let bytes = canonical_bytes(expected_authority, topology, registry)?;
        FleetRegistryVersion {
            authority: registry.authority.clone(),
            revision: registry.revision,
            content_hash: Sha256::digest(bytes).into(),
        }
    };
    Ok(FleetDirectorySnapshot {
        provenance: FleetDirectoryProvenance {
            registry: manifest,
            source_fleet_subnet_root,
        },
        fleet_subnet_roots: registry
            .fleet_subnet_roots
            .iter()
            .map(|entry| FleetSubnetRootDirectoryEntry {
                placement_subnet: entry.placement_subnet,
                fleet_subnet_root: entry.fleet_subnet_root,
                status: entry.status,
            })
            .collect(),
    })
}

fn compile_joining(
    expected_authority: &FleetRegistryAuthority,
    topology: &ComponentTopology,
    current: &FleetRegistry,
    entry: FleetSubnetRootEntry,
) -> Result<FleetRegistry, FleetRegistryOpsError> {
    validation::validate(expected_authority, topology, current)?;
    if entry.status != FleetSubnetRootStatus::Joining {
        return Err(FleetRegistryOpsError::FleetSubnetRootJoinRequiresJoining);
    }
    if current
        .fleet_subnet_roots
        .iter()
        .any(|existing| existing == &entry)
    {
        return Ok(current.clone());
    }
    if current.fleet_subnet_roots.iter().any(|existing| {
        existing.placement_subnet == entry.placement_subnet
            || existing.fleet_subnet_root == entry.fleet_subnet_root
    }) {
        return Err(FleetRegistryOpsError::FleetSubnetRootJoinIdentityConflict);
    }

    let mut next = current.clone();
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(FleetRegistryOpsError::RevisionExhausted)?;
    next.fleet_subnet_roots.push(entry);
    next.fleet_subnet_roots
        .sort_by_key(|root| root.placement_subnet);
    validation::validate(expected_authority, topology, &next)?;
    Ok(next)
}

fn compile_active(
    expected_authority: &FleetRegistryAuthority,
    topology: &ComponentTopology,
    current: &FleetRegistry,
) -> Result<FleetRegistry, FleetRegistryOpsError> {
    validation::validate(expected_authority, topology, current)?;
    if current.fleet_subnet_roots.is_empty()
        || current
            .fleet_subnet_roots
            .iter()
            .any(|entry| entry.status != FleetSubnetRootStatus::Joining)
    {
        return Err(FleetRegistryOpsError::FleetSubnetRootActivationRequiresAllJoining);
    }

    let mut next = current.clone();
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(FleetRegistryOpsError::RevisionExhausted)?;
    for root in &mut next.fleet_subnet_roots {
        root.status = FleetSubnetRootStatus::Active;
    }
    validation::validate(expected_authority, topology, &next)?;
    Ok(next)
}

fn compile_draining(
    expected_authority: &FleetRegistryAuthority,
    topology: &ComponentTopology,
    current: &FleetRegistry,
    fleet_subnet_root: Principal,
) -> Result<FleetRegistry, FleetRegistryOpsError> {
    validation::validate(expected_authority, topology, current)?;
    let target_index = current
        .fleet_subnet_roots
        .iter()
        .position(|entry| entry.fleet_subnet_root == fleet_subnet_root)
        .ok_or(FleetRegistryOpsError::FleetSubnetRootDrainingTargetMissing { fleet_subnet_root })?;
    if current.fleet_subnet_roots[target_index].status != FleetSubnetRootStatus::Active {
        return Err(FleetRegistryOpsError::FleetSubnetRootDrainingRequiresActive);
    }

    let mut next = current.clone();
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(FleetRegistryOpsError::RevisionExhausted)?;
    next.fleet_subnet_roots[target_index].status = FleetSubnetRootStatus::Draining;
    validation::validate(expected_authority, topology, &next)?;
    Ok(next)
}

fn compile_removed(
    expected_authority: &FleetRegistryAuthority,
    topology: &ComponentTopology,
    current: &FleetRegistry,
    fleet_subnet_root: Principal,
) -> Result<FleetRegistry, FleetRegistryOpsError> {
    validation::validate(expected_authority, topology, current)?;
    let target_index = current
        .fleet_subnet_roots
        .iter()
        .position(|entry| entry.fleet_subnet_root == fleet_subnet_root)
        .ok_or(FleetRegistryOpsError::FleetSubnetRootRemovalTargetMissing { fleet_subnet_root })?;
    if current.fleet_subnet_roots[target_index].status != FleetSubnetRootStatus::Draining {
        return Err(FleetRegistryOpsError::FleetSubnetRootRemovalRequiresDraining);
    }

    let mut next = current.clone();
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(FleetRegistryOpsError::RevisionExhausted)?;
    next.fleet_subnet_roots[target_index].status = FleetSubnetRootStatus::Removed;
    validation::validate(expected_authority, topology, &next)?;
    Ok(next)
}

fn canonical_bytes(
    expected_authority: &FleetRegistryAuthority,
    topology: &ComponentTopology,
    registry: &FleetRegistry,
) -> Result<Vec<u8>, FleetRegistryOpsError> {
    validation::validate(expected_authority, topology, registry)?;

    let mut encoder = CanonicalEncoder::new();
    encode_authority(&mut encoder, &registry.authority);
    encoder.u64(registry.revision);
    encoder.u64(registry.component_specs.len() as u64);
    for component_spec in &registry.component_specs {
        encode_component_spec(&mut encoder, component_spec);
    }
    encoder.u64(registry.fleet_subnet_roots.len() as u64);
    for root in &registry.fleet_subnet_roots {
        encode_root(&mut encoder, root);
    }
    encoder.finish()
}

fn encode_authority(encoder: &mut CanonicalEncoder, authority: &FleetRegistryAuthority) {
    let binding = &authority.binding;
    encoder.bytes(binding.fleet.fleet.canonical_network_id.as_bytes());
    encoder.bytes(binding.fleet.fleet.fleet_id.as_bytes());
    encoder.string(binding.fleet.app.as_str());
    encoder.bytes(binding.coordinator_subnet.as_principal().as_slice());
    encoder.bytes(binding.coordinator.as_slice());
    encoder.u64(authority.epoch);
}

fn encode_component_spec(encoder: &mut CanonicalEncoder, entry: &FleetComponentSpecEntry) {
    encoder.string(entry.component_spec.as_str());
    encoder.bytes(&entry.spec_hash);
    encoder.string(entry.component_role.as_str());
    encoder.u32(entry.maximum_fleet_instances);
}

fn encode_root(encoder: &mut CanonicalEncoder, root: &FleetSubnetRootEntry) {
    encoder.bytes(root.placement_subnet.as_principal().as_slice());
    encoder.bytes(root.fleet_subnet_root.as_slice());
    encoder.u64(root.component_admissions.len() as u64);
    for admission in &root.component_admissions {
        encode_admission(encoder, admission);
    }
    encoder.bytes(root.component_topology_digest.as_bytes());
    encoder.bytes(root.active_release_set.release_build_id.as_bytes());
    encoder.bytes(root.active_release_set.manifest_digest.as_bytes());
    encode_limits(encoder, &root.limits);
    encoder.u8(status_tag(root.status));
}

fn encode_admission(encoder: &mut CanonicalEncoder, admission: &ComponentSpecAdmission) {
    encoder.string(admission.component_spec.as_str());
    encoder.bytes(&admission.spec_hash);
    encoder.u32(admission.maximum_root_instances);
}

fn encode_limits(encoder: &mut CanonicalEncoder, limits: &FleetSubnetRootLimits) {
    encoder.u32(limits.maximum_component_instances);
    encoder.u32(limits.maximum_managed_canisters);
    encoder.u64(limits.maximum_registry_bytes);
    encoder.u64(limits.maximum_wasm_store_bytes);
    encoder.u32(limits.canister_pool.minimum_size);
    encoder.u32(limits.canister_pool.maximum_size);
    encoder.u128(limits.canister_pool.canister_cycles.to_u128());
    encoder.u64(limits.cycles_funding.window_secs);
    encoder.u128(limits.cycles_funding.maximum_cycles.to_u128());
}

const fn status_tag(status: FleetSubnetRootStatus) -> u8 {
    match status {
        FleetSubnetRootStatus::Joining => 0,
        FleetSubnetRootStatus::Active => 1,
        FleetSubnetRootStatus::Draining => 2,
        FleetSubnetRootStatus::Removed => 3,
    }
}

struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    fn new() -> Self {
        let mut encoder = Self { bytes: Vec::new() };
        encoder.bytes(FLEET_REGISTRY_DOMAIN);
        encoder.u32(FLEET_REGISTRY_SCHEMA_VERSION);
        encoder
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn finish(self) -> Result<Vec<u8>, FleetRegistryOpsError> {
        if self.bytes.len() > MAX_FLEET_REGISTRY_CANONICAL_BYTES {
            return Err(FleetRegistryOpsError::CanonicalBytesExceeded {
                actual_bytes: self.bytes.len(),
                maximum_bytes: MAX_FLEET_REGISTRY_CANONICAL_BYTES,
            });
        }
        Ok(self.bytes)
    }
}
