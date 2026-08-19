//! Module: workflow::fleet_registry_mirror
//!
//! Responsibility: stage Joining evidence and atomically activate complete current mirrors.
//! Does not own: Coordinator Registry mutation or runtime activation.
//! Boundary: each monotonic commit follows exact root, Store, Registry, and Directory validation.

use crate::{
    dto::root::RootRegistrySynchronizationOperationStatus,
    ops::{
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
    },
    view::fleet_registry_mirror::RootFleetRegistryActiveView,
    workflow::{
        bootstrap::root_store, fleet_coordinator_client, root_authority::validated_root_authority,
    },
};
use canic_core::{
    api::timer::TimerApi,
    control_plane_support::{
        error::InternalError,
        ops::{config::ConfigOps, fleet_registry::FleetRegistryOps},
    },
    dto::{
        fleet_registry::{
            FleetDirectorySnapshot, FleetRegistryManifest, FleetRegistrySnapshotResponse,
            FleetRegistryVersion, FleetSubnetRootEntry,
            FleetSubnetRootRegistryMirrorActivationRequest,
            FleetSubnetRootRegistryMirrorActivationResponse, FleetSubnetRootRegistrySyncRequest,
            FleetSubnetRootRegistrySyncResponse, FleetSubnetRootSnapshotAcknowledgement,
            FleetSubnetRootSnapshotAcknowledgementRequest, FleetSubnetRootStatus,
        },
        role::OperationReceipt,
    },
};
use std::time::Duration;

/// Validated source and target authority retained before one mirror commit.
pub(super) struct PreparedComponentPublicationTransition {
    pub source: FleetRegistrySnapshotResponse,
    pub target: FleetRegistrySnapshotResponse,
    pub directory: FleetDirectorySnapshot,
    previous_registry: FleetRegistryVersion,
}

/// Fetch and acknowledge the current all-Joining snapshot.
pub async fn synchronize(
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<FleetSubnetRootRegistrySyncResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;

    let snapshot = fetch_snapshot(&authority).await?;
    validate_snapshot(&authority, &snapshot, FleetSubnetRootStatus::Joining)?;
    if snapshot.version != request.expected_registry {
        return Err(InternalError::conflict());
    }
    let mirror = FleetRegistryMirrorOps::current();
    if mirror.active.is_some() {
        return Err(InternalError::conflict());
    }
    let candidate = mirror.candidate;
    if let Some(existing) = &candidate {
        if existing.operation_id != request.operation_id
            || existing.store_bootstrap != request.store_bootstrap
            || existing.snapshot != snapshot
        {
            return Err(InternalError::conflict());
        }
        if let Some(acknowledgement) = &existing.acknowledgement {
            return response(root, &snapshot, acknowledgement.clone());
        }
    } else {
        FleetRegistryMirrorOps::commit_candidate(
            request.operation_id,
            request.store_bootstrap.clone(),
            snapshot.clone(),
            None,
        );
    }

    let acknowledgement = acknowledge_snapshot(
        authority.binding.authority.binding.coordinator,
        snapshot.version.clone(),
    )
    .await?;
    if acknowledgement.fleet_subnet_root != root || acknowledgement.version != snapshot.version {
        return Err(InternalError::invalid_input());
    }
    FleetRegistryMirrorOps::commit_candidate(
        request.operation_id,
        request.store_bootstrap.clone(),
        snapshot.clone(),
        Some(acknowledgement.clone()),
    );
    response(root, &snapshot, acknowledgement)
}

/// Accept or exactly replay one synchronization intent and ensure its reconciler is scheduled.
pub async fn accept_synchronization(
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<OperationReceipt, InternalError> {
    let mirror = FleetRegistryMirrorOps::current();
    let existing = mirror
        .candidate
        .as_ref()
        .or(mirror.synchronization.as_ref());
    if let Some(existing) = existing {
        let replay_is_exact = existing.operation_id == request.operation_id
            && existing.store_bootstrap == request.store_bootstrap
            && existing.snapshot.version == request.expected_registry;
        if !replay_is_exact {
            return Err(InternalError::conflict());
        }
    } else {
        synchronize(request.clone()).await?;
    }
    schedule_registry_synchronization(request.operation_id);
    Ok(OperationReceipt {
        operation_id: request.operation_id,
    })
}

/// Revalidate durable local evidence without changing it.
pub async fn status(
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<FleetSubnetRootRegistrySyncResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    let candidate = FleetRegistryMirrorOps::current()
        .candidate
        .ok_or_else(InternalError::unavailable)?;
    if candidate.operation_id != request.operation_id
        || candidate.store_bootstrap != request.store_bootstrap
    {
        return Err(InternalError::conflict());
    }
    validate_snapshot(
        &authority,
        &candidate.snapshot,
        FleetSubnetRootStatus::Joining,
    )?;
    if candidate.snapshot.version != request.expected_registry {
        return Err(InternalError::conflict());
    }
    let acknowledgement = candidate
        .acknowledgement
        .ok_or_else(InternalError::unavailable)?;
    response(root, &candidate.snapshot, acknowledgement)
}

/// Resolve the initial Registry synchronization through its retained operation receipt.
pub fn synchronization_operation_status(
    operation_id: [u8; 32],
) -> Result<Option<RootRegistrySynchronizationOperationStatus>, InternalError> {
    let mirror = FleetRegistryMirrorOps::current();
    let candidate = mirror
        .candidate
        .filter(|candidate| candidate.operation_id == operation_id)
        .or_else(|| {
            mirror
                .synchronization
                .filter(|candidate| candidate.operation_id == operation_id)
        });
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    let acknowledgement = candidate
        .acknowledgement
        .ok_or_else(InternalError::unavailable)?;
    let synchronization = FleetSubnetRootRegistrySyncResponse {
        fleet_subnet_root: acknowledgement.fleet_subnet_root,
        version: candidate.snapshot.version,
        acknowledgement,
    };
    let activation = mirror
        .active
        .map(|active| active_response(synchronization.fleet_subnet_root, &active));
    Ok(Some(RootRegistrySynchronizationOperationStatus {
        synchronization,
        activation,
    }))
}

/// Privately wait for and activate the Coordinator's all-Active Registry.
pub fn schedule_registry_synchronization(operation_id: [u8; 32]) {
    schedule_registry_synchronization_after(operation_id, Duration::ZERO);
}

fn schedule_registry_synchronization_after(operation_id: [u8; 32], delay: Duration) {
    TimerApi::defer_lifecycle_required(
        delay,
        "Fleet Subnet Root Registry synchronization",
        async move {
            match advance_registry_synchronization_once(operation_id).await {
                Ok(true) => {}
                Ok(false) => {
                    schedule_registry_synchronization_after(operation_id, Duration::ZERO);
                }
                Err(_) => {
                    schedule_registry_synchronization_after(operation_id, Duration::from_secs(1));
                }
            }
        },
    );
}

async fn advance_registry_synchronization_once(
    operation_id: [u8; 32],
) -> Result<bool, InternalError> {
    let mirror = FleetRegistryMirrorOps::current();
    if mirror
        .synchronization
        .as_ref()
        .is_some_and(|synchronization| synchronization.operation_id == operation_id)
        && mirror.active.is_some()
    {
        return Ok(true);
    }
    let candidate = mirror.candidate.ok_or_else(InternalError::unavailable)?;
    if candidate.operation_id != operation_id {
        return Err(InternalError::conflict());
    }
    let (authority, root) = validated_root_authority()?;
    let target = fetch_snapshot(&authority).await?;
    validate_snapshot(&authority, &target, FleetSubnetRootStatus::Active)?;
    let expected_directory = FleetRegistryOps::directory_for_root(
        &authority.binding.authority,
        &ConfigOps::component_topology()?,
        &target.registry,
        root,
    )?;
    activate(FleetSubnetRootRegistryMirrorActivationRequest {
        previous_registry: candidate.snapshot.version,
        expected_registry: target.version,
        expected_directory,
        store_bootstrap: candidate.store_bootstrap,
    })
    .await?;
    Ok(true)
}

/// Atomically activate the requested complete mirror and Directory without allowing regression.
pub async fn activate(
    request: FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    validate_transition_request(&authority, &request)?;

    if let Some(current) = validated_active_if_present(&authority, root)? {
        match classify_active_transition(&current, &request)? {
            ActiveMirrorTransition::Current => return Ok(active_response(root, &current)),
            ActiveMirrorTransition::Advance => {}
        }
    }

    let snapshot = fetch_snapshot(&authority).await?;
    let directory = validate_target(&authority, root, &request, &snapshot)?;
    let mirror = FleetRegistryMirrorOps::current();

    if mirror.active.is_some() {
        let current = validated_active(&authority, root)?;
        return match classify_active_transition(&current, &request)? {
            ActiveMirrorTransition::Current => Ok(active_response(root, &current)),
            ActiveMirrorTransition::Advance => {
                FleetRegistryMirrorOps::commit_active(
                    request.previous_registry,
                    snapshot,
                    directory,
                );
                let active = validated_active(&authority, root)?;
                Ok(active_response(root, &active))
            }
        };
    }
    let candidate = mirror.candidate.ok_or_else(InternalError::unavailable)?;
    validate_snapshot(
        &authority,
        &candidate.snapshot,
        FleetSubnetRootStatus::Joining,
    )?;
    if candidate.snapshot.version != request.previous_registry {
        return Err(InternalError::conflict());
    }
    let acknowledgement = candidate
        .acknowledgement
        .ok_or_else(InternalError::unavailable)?;
    if acknowledgement.fleet_subnet_root != root
        || acknowledgement.version != request.previous_registry
    {
        return Err(InternalError::invariant());
    }

    FleetRegistryMirrorOps::commit_active(request.previous_registry, snapshot, directory);
    let active = validated_active(&authority, root)?;
    Ok(active_response(root, &active))
}

/// Independently revalidate the durable active mirror and Directory without mutation.
pub async fn active_status(
    request: FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    validate_transition_request(&authority, &request)?;
    let active = validated_active(&authority, root)?;
    match classify_active_transition(&active, &request)? {
        ActiveMirrorTransition::Current => Ok(active_response(root, &active)),
        ActiveMirrorTransition::Advance => Err(InternalError::unavailable()),
    }
}

/// Advance one Prepared root to the exact Coordinator-published service Registry.
pub async fn advance_for_component_publication(
    previous_registry: FleetRegistryVersion,
    expected_registry: FleetRegistryVersion,
    store_bootstrap: canic_core::dto::root_store::RootStoreBootstrapRequest,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    root_store::status(store_bootstrap.clone()).await?;
    if previous_registry == expected_registry {
        let active = validated_active(&authority, root)?;
        if active.snapshot.version != expected_registry {
            return Err(InternalError::conflict());
        }
        return Ok(active_response(root, &active));
    }
    let snapshot = fetch_snapshot(&authority).await?;
    let expected_directory = FleetRegistryOps::directory_for_root(
        &authority.binding.authority,
        &ConfigOps::component_topology()?,
        &snapshot.registry,
        root,
    )?;
    let request = FleetSubnetRootRegistryMirrorActivationRequest {
        previous_registry,
        expected_registry,
        expected_directory,
        store_bootstrap,
    };
    validate_transition_request(&authority, &request)?;
    let directory = validate_target(&authority, root, &request, &snapshot)?;
    let current = validated_active(&authority, root)?;
    match classify_active_transition(&current, &request)? {
        ActiveMirrorTransition::Current => Ok(active_response(root, &current)),
        ActiveMirrorTransition::Advance => {
            FleetRegistryMirrorOps::commit_active(request.previous_registry, snapshot, directory);
            let active = validated_active(&authority, root)?;
            Ok(active_response(root, &active))
        }
    }
}

/// Advance the local mirror to the Coordinator-published Draining root entry.
///
/// Root removal owns this refresh privately after the high-level removal intent
/// is accepted. It does not expose another phase-selecting endpoint.
pub(super) async fn advance_to_draining_for_root_removal(
    store_bootstrap: canic_core::dto::root_store::RootStoreBootstrapRequest,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    root_store::status(store_bootstrap.clone()).await?;
    let current = validated_active(&authority, root)?;
    let target = fetch_snapshot(&authority).await?;
    validate_snapshot(&authority, &target, FleetSubnetRootStatus::Draining)?;
    advance_for_component_publication(current.snapshot.version, target.version, store_bootstrap)
        .await
}

/// Fetch and validate one exact publication target without mutating the local mirror.
pub(super) async fn prepare_component_publication_transition(
    previous_registry: FleetRegistryVersion,
    expected_registry: FleetRegistryVersion,
    store_bootstrap: canic_core::dto::root_store::RootStoreBootstrapRequest,
) -> Result<PreparedComponentPublicationTransition, InternalError> {
    let (authority, root) = validated_root_authority()?;
    root_store::status(store_bootstrap.clone()).await?;
    let current = validated_active(&authority, root)?;
    if current.snapshot.version != previous_registry {
        return Err(InternalError::conflict());
    }
    let target = if previous_registry == expected_registry {
        current.snapshot.clone()
    } else {
        fetch_snapshot(&authority).await?
    };
    let directory = FleetRegistryOps::directory_for_root(
        &authority.binding.authority,
        &ConfigOps::component_topology()?,
        &target.registry,
        root,
    )?;
    let request = FleetSubnetRootRegistryMirrorActivationRequest {
        previous_registry: previous_registry.clone(),
        expected_registry,
        expected_directory: directory,
        store_bootstrap,
    };
    validate_transition_request(&authority, &request)?;
    let directory = validate_target(&authority, root, &request, &target)?;
    Ok(PreparedComponentPublicationTransition {
        source: current.snapshot,
        target,
        directory,
        previous_registry,
    })
}

/// Commit one previously validated exact publication target without another await.
pub(super) fn commit_component_publication_transition(
    prepared: &PreparedComponentPublicationTransition,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    let current = validated_active(&authority, root)?;
    let snapshot_is_target = current.snapshot == prepared.target;
    let directory_is_target = current.directory == prepared.directory;
    if snapshot_is_target && directory_is_target {
        return Ok(active_response(root, &current));
    }
    if current.snapshot != prepared.source || current.snapshot.version != prepared.previous_registry
    {
        return Err(InternalError::conflict());
    }
    FleetRegistryMirrorOps::commit_active(
        prepared.previous_registry.clone(),
        prepared.target.clone(),
        prepared.directory.clone(),
    );
    let active = validated_active(&authority, root)?;
    Ok(active_response(root, &active))
}

async fn fetch_snapshot(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
) -> Result<FleetRegistrySnapshotResponse, InternalError> {
    let registry =
        fleet_coordinator_client::registry(authority.binding.authority.binding.coordinator).await?;
    let topology = ConfigOps::component_topology()?;
    let manifest = FleetRegistryOps::manifest(&authority.binding.authority, &topology, &registry)?;
    let version = FleetRegistryVersion {
        authority: manifest.authority.clone(),
        revision: manifest.revision,
        content_hash: manifest.content_hash,
    };
    Ok(FleetRegistrySnapshotResponse {
        registry,
        manifest,
        version,
    })
}

async fn acknowledge_snapshot(
    coordinator: candid::Principal,
    version: canic_core::dto::fleet_registry::FleetRegistryVersion,
) -> Result<FleetSubnetRootSnapshotAcknowledgement, InternalError> {
    fleet_coordinator_client::acknowledge_root_snapshot(
        coordinator,
        FleetSubnetRootSnapshotAcknowledgementRequest { version },
    )
    .await
}

fn validate_snapshot(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    snapshot: &FleetRegistrySnapshotResponse,
    expected_status: FleetSubnetRootStatus,
) -> Result<(), InternalError> {
    let root_entry = validated_snapshot_root(authority, snapshot)?;
    if root_entry.status != expected_status {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct CanonicalSnapshotEvidence<'a> {
    manifest: &'a FleetRegistryManifest,
    version: &'a FleetRegistryVersion,
}

fn validated_snapshot_root(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    snapshot: &FleetRegistrySnapshotResponse,
) -> Result<FleetSubnetRootEntry, InternalError> {
    let topology = ConfigOps::component_topology()?;
    FleetRegistryOps::validate(&authority.binding.authority, &topology, &snapshot.registry)?;
    let manifest =
        FleetRegistryOps::manifest(&authority.binding.authority, &topology, &snapshot.registry)?;
    let version =
        FleetRegistryOps::version(&authority.binding.authority, &topology, &snapshot.registry)?;
    let root_entry = snapshot
        .registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == authority.binding.fleet_subnet_root)
        .cloned()
        .ok_or_else(InternalError::invalid_input)?;
    let expected = FleetSubnetRootEntry {
        placement_subnet: authority.binding.placement_subnet,
        fleet_subnet_root: authority.binding.fleet_subnet_root,
        component_admissions: authority.binding.component_admissions.clone(),
        component_topology_digest: authority.binding.component_topology_digest,
        active_release_set: authority.initial_release_set,
        limits: authority.binding.limits.clone(),
        status: root_entry.status,
    };
    let supplied = CanonicalSnapshotEvidence {
        manifest: &snapshot.manifest,
        version: &snapshot.version,
    };
    let canonical = CanonicalSnapshotEvidence {
        manifest: &manifest,
        version: &version,
    };
    if supplied != canonical || root_entry != expected {
        return Err(InternalError::invalid_input());
    }
    Ok(root_entry)
}

fn validate_target(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    root: candid::Principal,
    request: &FleetSubnetRootRegistryMirrorActivationRequest,
    snapshot: &FleetRegistrySnapshotResponse,
) -> Result<FleetDirectorySnapshot, InternalError> {
    let root_entry = validated_snapshot_root(authority, snapshot)?;
    if snapshot.version != request.expected_registry {
        return Err(InternalError::conflict());
    }
    let topology = ConfigOps::component_topology()?;
    let directory = FleetRegistryOps::directory_for_root(
        &authority.binding.authority,
        &topology,
        &snapshot.registry,
        root,
    )?;
    if directory != request.expected_directory {
        return Err(InternalError::conflict());
    }
    if root_entry.status == FleetSubnetRootStatus::Draining {
        ComponentRegistryOps::validate_published_root_draining(&snapshot.version)?;
    }
    Ok(directory)
}

fn validate_transition_request(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    request: &FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<(), InternalError> {
    let expected_authority = &authority.binding.authority;
    let authority_is_exact = request.previous_registry.authority == *expected_authority
        && request.expected_registry.authority == *expected_authority;
    let revision_advances = request.previous_registry.revision < request.expected_registry.revision;
    let hashes_are_present = request.previous_registry.content_hash != [0; 32]
        && request.expected_registry.content_hash != [0; 32];
    if ![authority_is_exact, revision_advances, hashes_are_present]
        .into_iter()
        .all(|valid| valid)
    {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
enum ActiveMirrorTransition {
    Current,
    Advance,
}

#[derive(Eq, PartialEq)]
struct ActiveMirrorAuthority<'a> {
    previous_registry: &'a FleetRegistryVersion,
    registry: &'a FleetRegistryVersion,
    directory: &'a FleetDirectorySnapshot,
}

impl<'a> ActiveMirrorAuthority<'a> {
    const fn from_active(active: &'a RootFleetRegistryActiveView) -> Self {
        Self {
            previous_registry: &active.previous_registry,
            registry: &active.snapshot.version,
            directory: &active.directory,
        }
    }

    const fn from_request(request: &'a FleetSubnetRootRegistryMirrorActivationRequest) -> Self {
        Self {
            previous_registry: &request.previous_registry,
            registry: &request.expected_registry,
            directory: &request.expected_directory,
        }
    }
}

fn classify_active_transition(
    active: &RootFleetRegistryActiveView,
    request: &FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<ActiveMirrorTransition, InternalError> {
    let current = &active.snapshot.version;
    if ActiveMirrorAuthority::from_active(active) == ActiveMirrorAuthority::from_request(request) {
        return Ok(ActiveMirrorTransition::Current);
    }
    if current == &request.expected_registry {
        return Err(InternalError::conflict());
    }
    if current.authority != request.expected_registry.authority {
        return Err(InternalError::conflict());
    }
    if current.revision > request.expected_registry.revision {
        return Ok(ActiveMirrorTransition::Current);
    }
    if current == &request.previous_registry {
        return Ok(ActiveMirrorTransition::Advance);
    }
    Err(InternalError::conflict())
}

fn validated_active_if_present(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    root: candid::Principal,
) -> Result<Option<RootFleetRegistryActiveView>, InternalError> {
    if FleetRegistryMirrorOps::current().active.is_none() {
        return Ok(None);
    }
    validated_active(authority, root).map(Some)
}

fn validated_active(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    root: candid::Principal,
) -> Result<RootFleetRegistryActiveView, InternalError> {
    let mirror = FleetRegistryMirrorOps::validated_current(authority, root)?;
    if mirror.root_entry.status == FleetSubnetRootStatus::Draining {
        ComponentRegistryOps::validate_published_root_draining(&mirror.active.snapshot.version)?;
    }
    Ok(mirror.active)
}

fn active_response(
    root: candid::Principal,
    active: &RootFleetRegistryActiveView,
) -> FleetSubnetRootRegistryMirrorActivationResponse {
    FleetSubnetRootRegistryMirrorActivationResponse {
        fleet_subnet_root: root,
        previous_registry: active.previous_registry.clone(),
        version: active.snapshot.version.clone(),
        directory: active.directory.clone(),
    }
}

fn response(
    root: candid::Principal,
    snapshot: &FleetRegistrySnapshotResponse,
    acknowledgement: FleetSubnetRootSnapshotAcknowledgement,
) -> Result<FleetSubnetRootRegistrySyncResponse, InternalError> {
    if acknowledgement.fleet_subnet_root != root || acknowledgement.version != snapshot.version {
        return Err(InternalError::invariant());
    }
    Ok(FleetSubnetRootRegistrySyncResponse {
        fleet_subnet_root: root,
        version: snapshot.version.clone(),
        acknowledgement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        dto::{
            fleet_registry::{FleetDirectoryProvenance, FleetRegistry},
            root_store::RootStoreBootstrapRequest,
        },
        ids::{
            AppId, CanonicalNetworkId, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, SubnetId,
        },
    };

    #[test]
    fn transition_classification_is_exact_monotonic_and_stale_convergent() {
        let active = active_mirror(5, 8);

        assert_eq!(
            classify_active_transition(&active, &request(5, 8)).expect("exact retry"),
            ActiveMirrorTransition::Current
        );
        assert_eq!(
            classify_active_transition(&active, &request(4, 7)).expect("stale retry"),
            ActiveMirrorTransition::Current
        );
        assert_eq!(
            classify_active_transition(&active, &request(8, 9)).expect("exact advance"),
            ActiveMirrorTransition::Advance
        );
        assert!(classify_active_transition(&active, &request(7, 9)).is_err());
        assert!(classify_active_transition(&active, &request(4, 8)).is_err());
    }

    fn active_mirror(previous_revision: u64, current_revision: u64) -> RootFleetRegistryActiveView {
        let previous_registry = version(previous_revision);
        let current = version(current_revision);
        RootFleetRegistryActiveView {
            previous_registry,
            snapshot: FleetRegistrySnapshotResponse {
                registry: FleetRegistry {
                    authority: current.authority.clone(),
                    revision: current.revision,
                    component_specs: Vec::new(),
                    fleet_subnet_roots: Vec::new(),
                    services: Vec::new(),
                },
                manifest: FleetRegistryManifest {
                    authority: current.authority.clone(),
                    revision: current.revision,
                    byte_length: 1,
                    content_hash: current.content_hash,
                },
                version: current.clone(),
            },
            directory: directory(current),
        }
    }

    fn request(
        previous_revision: u64,
        expected_revision: u64,
    ) -> FleetSubnetRootRegistryMirrorActivationRequest {
        let expected_registry = version(expected_revision);
        FleetSubnetRootRegistryMirrorActivationRequest {
            previous_registry: version(previous_revision),
            expected_directory: directory(expected_registry.clone()),
            expected_registry,
            store_bootstrap: RootStoreBootstrapRequest {
                operation_id: [6; 32],
                manifest_payload_size_bytes: 1,
            },
        }
    }

    fn directory(registry: FleetRegistryVersion) -> FleetDirectorySnapshot {
        FleetDirectorySnapshot {
            provenance: FleetDirectoryProvenance {
                registry,
                source_fleet_subnet_root: candid::Principal::from_slice(&[4; 29]),
            },
            fleet_subnet_roots: Vec::new(),
            services: Vec::new(),
        }
    }

    fn version(revision: u64) -> FleetRegistryVersion {
        FleetRegistryVersion {
            authority: authority(),
            revision,
            content_hash: [u8::try_from(revision).expect("test revision fits in one byte"); 32],
        }
    }

    fn authority() -> FleetRegistryAuthority {
        FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([1; 32]),
                    },
                    app: AppId::owned("test".to_owned()),
                },
                coordinator_subnet: SubnetId::from_principal(candid::Principal::from_slice(
                    &[2; 29],
                )),
                coordinator: candid::Principal::from_slice(&[3; 29]),
            },
            epoch: 1,
        }
    }
}
