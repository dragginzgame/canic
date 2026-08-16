//! Module: workflow::fleet_registry_mirror
//!
//! Responsibility: stage Joining evidence and atomically activate complete current mirrors.
//! Does not own: Coordinator Registry mutation or runtime activation.
//! Boundary: each monotonic commit follows exact root, Store, Registry, and Directory validation.

use crate::{
    ops::{
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
    },
    view::fleet_registry_mirror::RootFleetRegistryActiveView,
    workflow::{bootstrap::root_store, root_authority::validated_root_authority},
};
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::{config::ConfigOps, fleet_registry::FleetRegistryOps, ic::call::CallOps},
    },
    dto::{
        error::Error,
        fleet_registry::{
            FleetDirectorySnapshot, FleetRegistryManifest, FleetRegistrySnapshotResponse,
            FleetRegistryVersion, FleetSubnetRootEntry,
            FleetSubnetRootRegistryMirrorActivationRequest,
            FleetSubnetRootRegistryMirrorActivationResponse, FleetSubnetRootRegistrySyncRequest,
            FleetSubnetRootRegistrySyncResponse, FleetSubnetRootSnapshotAcknowledgement,
            FleetSubnetRootSnapshotAcknowledgementRequest, FleetSubnetRootStatus,
        },
    },
    protocol,
};

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

    let snapshot = fetch_snapshot(authority.binding.authority.binding.coordinator).await?;
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
        if existing.snapshot != snapshot {
            return Err(InternalError::conflict());
        }
        if let Some(acknowledgement) = &existing.acknowledgement {
            return response(root, &snapshot, acknowledgement.clone());
        }
    } else {
        FleetRegistryMirrorOps::commit_candidate(snapshot.clone(), None);
    }

    let acknowledgement = acknowledge_snapshot(
        authority.binding.authority.binding.coordinator,
        snapshot.version.clone(),
    )
    .await?;
    if acknowledgement.fleet_subnet_root != root || acknowledgement.version != snapshot.version {
        return Err(InternalError::invalid_input());
    }
    FleetRegistryMirrorOps::commit_candidate(snapshot.clone(), Some(acknowledgement.clone()));
    response(root, &snapshot, acknowledgement)
}

/// Revalidate durable local evidence without changing it.
pub async fn status(
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<FleetSubnetRootRegistrySyncResponse, InternalError> {
    let (authority, root) = validated_root_authority()?;
    root_store::status(request.store_bootstrap).await?;
    let candidate = FleetRegistryMirrorOps::current()
        .candidate
        .ok_or_else(|| InternalError::unavailable())?;
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
        .ok_or_else(|| InternalError::unavailable())?;
    response(root, &candidate.snapshot, acknowledgement)
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

    let snapshot = fetch_snapshot(authority.binding.authority.binding.coordinator).await?;
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
    let candidate = mirror
        .candidate
        .ok_or_else(|| InternalError::unavailable())?;
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
        .ok_or_else(|| InternalError::unavailable())?;
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
    let snapshot = fetch_snapshot(authority.binding.authority.binding.coordinator).await?;
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
        fetch_snapshot(authority.binding.authority.binding.coordinator).await?
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
    coordinator: candid::Principal,
) -> Result<FleetRegistrySnapshotResponse, InternalError> {
    let call = CallOps::unbounded_wait(
        coordinator,
        protocol::CANIC_FLEET_REGISTRY_SNAPSHOT_FOR_ROOT,
    )
    .execute()
    .await?;
    let result: Result<FleetRegistrySnapshotResponse, Error> = call.candid()?;
    result.map_err(InternalError::observed_public)
}

async fn acknowledge_snapshot(
    coordinator: candid::Principal,
    version: canic_core::dto::fleet_registry::FleetRegistryVersion,
) -> Result<FleetSubnetRootSnapshotAcknowledgement, InternalError> {
    let call =
        CallOps::unbounded_wait(coordinator, protocol::CANIC_FLEET_REGISTRY_ACKNOWLEDGE_ROOT)
            .with_arg(FleetSubnetRootSnapshotAcknowledgementRequest { version })?
            .execute()
            .await?;
    let result: Result<FleetSubnetRootSnapshotAcknowledgement, Error> = call.candid()?;
    result.map_err(InternalError::observed_public)
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
        .ok_or_else(|| InternalError::invalid_input())?;
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
