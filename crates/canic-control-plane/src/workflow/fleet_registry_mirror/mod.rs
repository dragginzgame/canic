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
    workflow::bootstrap::root_store,
};
use canic_core::{
    api::fleet_activation::FleetActivationApi,
    control_plane_support::{
        error::InternalError,
        ops::{
            config::ConfigOps,
            fleet_registry::FleetRegistryOps,
            ic::{IcOps, call::CallOps},
        },
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

/// Fetch and acknowledge the current all-Joining snapshot.
pub async fn synchronize(
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<FleetSubnetRootRegistrySyncResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;

    let snapshot = fetch_snapshot(authority.binding.authority.binding.coordinator).await?;
    validate_snapshot(&authority, &snapshot, FleetSubnetRootStatus::Joining)?;
    if snapshot.version != request.expected_registry {
        return Err(InternalError::conflict(
            "Coordinator snapshot differs from the host-expected Registry version",
        ));
    }
    let mirror = FleetRegistryMirrorOps::current();
    if mirror.active.is_some() {
        return Err(InternalError::conflict(
            "root already contains an active Fleet Registry mirror",
        ));
    }
    let candidate = mirror.candidate;
    if let Some(existing) = &candidate {
        if existing.snapshot != snapshot {
            return Err(InternalError::conflict(
                "root already contains different Fleet Registry candidate authority",
            ));
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
        return Err(InternalError::invalid_input(
            "Coordinator acknowledgement differs from the staged root snapshot",
        ));
    }
    FleetRegistryMirrorOps::commit_candidate(snapshot.clone(), Some(acknowledgement.clone()));
    response(root, &snapshot, acknowledgement)
}

/// Revalidate durable local evidence without changing it.
pub async fn status(
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<FleetSubnetRootRegistrySyncResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap).await?;
    let candidate = FleetRegistryMirrorOps::current()
        .candidate
        .ok_or_else(|| InternalError::unavailable("root has no staged Fleet Registry snapshot"))?;
    validate_snapshot(
        &authority,
        &candidate.snapshot,
        FleetSubnetRootStatus::Joining,
    )?;
    if candidate.snapshot.version != request.expected_registry {
        return Err(InternalError::conflict(
            "stored root snapshot differs from the host-expected Registry version",
        ));
    }
    let acknowledgement = candidate.acknowledgement.ok_or_else(|| {
        InternalError::unavailable("root snapshot has not been acknowledged by the Coordinator")
    })?;
    response(root, &candidate.snapshot, acknowledgement)
}

/// Atomically activate the requested complete mirror and Directory without allowing regression.
pub async fn activate(
    request: FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    let (authority, root) = root_authority()?;
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
    let candidate = mirror.candidate.ok_or_else(|| {
        InternalError::unavailable("root has no acknowledged Joining Registry candidate")
    })?;
    validate_snapshot(
        &authority,
        &candidate.snapshot,
        FleetSubnetRootStatus::Joining,
    )?;
    if candidate.snapshot.version != request.previous_registry {
        return Err(InternalError::conflict(
            "root Joining candidate differs from the requested previous Registry",
        ));
    }
    let acknowledgement = candidate.acknowledgement.ok_or_else(|| {
        InternalError::unavailable("root Joining candidate lacks its Coordinator acknowledgement")
    })?;
    if acknowledgement.fleet_subnet_root != root
        || acknowledgement.version != request.previous_registry
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "stored Joining acknowledgement differs from candidate authority",
        ));
    }

    FleetRegistryMirrorOps::commit_active(request.previous_registry, snapshot, directory);
    let active = validated_active(&authority, root)?;
    Ok(active_response(root, &active))
}

/// Independently revalidate the durable active mirror and Directory without mutation.
pub async fn active_status(
    request: FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    validate_transition_request(&authority, &request)?;
    let active = validated_active(&authority, root)?;
    match classify_active_transition(&active, &request)? {
        ActiveMirrorTransition::Current => Ok(active_response(root, &active)),
        ActiveMirrorTransition::Advance => Err(InternalError::unavailable(
            "root Fleet Registry mirror has not reached the requested version",
        )),
    }
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
    result.map_err(InternalError::public)
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
    result.map_err(InternalError::public)
}

fn validate_snapshot(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    snapshot: &FleetRegistrySnapshotResponse,
    expected_status: FleetSubnetRootStatus,
) -> Result<(), InternalError> {
    let root_entry = validated_snapshot_root(authority, snapshot)?;
    if root_entry.status != expected_status {
        return Err(InternalError::invalid_input(
            "Fleet Registry snapshot root lifecycle differs from required authority",
        ));
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
        .ok_or_else(|| {
            InternalError::invalid_input(
                "Fleet Registry snapshot does not contain the protected root",
            )
        })?;
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
        return Err(InternalError::invalid_input(
            "Fleet Registry snapshot differs from protected root authority",
        ));
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
        return Err(InternalError::conflict(
            "Coordinator snapshot differs from the controller-expected Registry version",
        ));
    }
    let topology = ConfigOps::component_topology()?;
    let directory = FleetRegistryOps::directory_for_root(
        &authority.binding.authority,
        &topology,
        &snapshot.registry,
        root,
    )?;
    if directory != request.expected_directory {
        return Err(InternalError::conflict(
            "derived Fleet Directory differs from controller-expected authority",
        ));
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
        return Err(InternalError::invalid_input(
            "Registry mirror request does not name one exact monotonic authority transition",
        ));
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
        return Err(InternalError::conflict(
            "current Registry mirror target is bound to different transition authority",
        ));
    }
    if current.authority != request.expected_registry.authority {
        return Err(InternalError::conflict(
            "current Registry mirror belongs to different authority",
        ));
    }
    if current.revision > request.expected_registry.revision {
        return Ok(ActiveMirrorTransition::Current);
    }
    if current == &request.previous_registry {
        return Ok(ActiveMirrorTransition::Advance);
    }
    Err(InternalError::conflict(
        "current Registry mirror does not match the requested transition source",
    ))
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

fn root_authority() -> Result<
    (
        canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
        candid::Principal,
    ),
    InternalError,
> {
    let authority = FleetActivationApi::root_authority().map_err(InternalError::public)?;
    let root = IcOps::canister_self();
    if authority.binding.fleet_subnet_root != root {
        return Err(InternalError::invalid_input(
            "protected Fleet Subnet Root authority does not name this Canister",
        ));
    }
    Ok((authority, root))
}

fn response(
    root: candid::Principal,
    snapshot: &FleetRegistrySnapshotResponse,
    acknowledgement: FleetSubnetRootSnapshotAcknowledgement,
) -> Result<FleetSubnetRootRegistrySyncResponse, InternalError> {
    if acknowledgement.fleet_subnet_root != root || acknowledgement.version != snapshot.version {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "stored root snapshot acknowledgement differs from candidate authority",
        ));
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
