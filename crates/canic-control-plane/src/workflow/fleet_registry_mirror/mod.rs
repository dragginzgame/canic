//! Module: workflow::fleet_registry_mirror
//!
//! Responsibility: stage Joining Registry evidence and atomically activate its all-Active mirror.
//! Does not own: Coordinator Registry mutation or runtime activation.
//! Boundary: each commit follows exact root, topology, Store, Registry, and Directory validation.

use crate::{
    ops::fleet_registry_mirror::FleetRegistryMirrorOps,
    view::fleet_registry_mirror::RootFleetRegistryActiveView, workflow::bootstrap::root_store,
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
            FleetDirectorySnapshot, FleetRegistrySnapshotResponse, FleetSubnetRootEntry,
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

/// Atomically replace the private Joining candidate with the all-Active mirror and Directory.
pub async fn activate(
    request: FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    let snapshot = fetch_snapshot(authority.binding.authority.binding.coordinator).await?;
    let directory = validate_active_target(&authority, root, &request, &snapshot)?;
    let mirror = FleetRegistryMirrorOps::current();

    if let Some(active) = mirror.active {
        return active_response(root, &request, &active, &snapshot, &directory);
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

    FleetRegistryMirrorOps::commit_active(
        request.previous_registry.clone(),
        snapshot,
        directory.clone(),
    );
    Ok(activation_response(root, &request, directory))
}

/// Independently revalidate the durable active mirror and Directory without mutation.
pub async fn active_status(
    request: FleetSubnetRootRegistryMirrorActivationRequest,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    let active = FleetRegistryMirrorOps::current()
        .active
        .ok_or_else(|| InternalError::unavailable("root has no active Fleet Registry mirror"))?;
    let directory = validate_active_target(&authority, root, &request, &active.snapshot)?;
    active_response(root, &request, &active, &active.snapshot, &directory)
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
    let topology = ConfigOps::component_topology()?;
    FleetRegistryOps::validate(&authority.binding.authority, &topology, &snapshot.registry)?;
    let manifest =
        FleetRegistryOps::manifest(&authority.binding.authority, &topology, &snapshot.registry)?;
    let version =
        FleetRegistryOps::version(&authority.binding.authority, &topology, &snapshot.registry)?;
    let expected = FleetSubnetRootEntry {
        placement_subnet: authority.binding.placement_subnet,
        fleet_subnet_root: authority.binding.fleet_subnet_root,
        component_admissions: authority.binding.component_admissions.clone(),
        component_topology_digest: authority.binding.component_topology_digest,
        active_release_set: authority.initial_release_set,
        limits: authority.binding.limits.clone(),
        status: expected_status,
    };
    if snapshot.manifest != manifest
        || snapshot.version != version
        || !snapshot
            .registry
            .fleet_subnet_roots
            .iter()
            .any(|entry| entry == &expected)
    {
        return Err(InternalError::invalid_input(
            "Fleet Registry snapshot differs from protected root authority",
        ));
    }
    Ok(())
}

fn validate_active_target(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    root: candid::Principal,
    request: &FleetSubnetRootRegistryMirrorActivationRequest,
    snapshot: &FleetRegistrySnapshotResponse,
) -> Result<FleetDirectorySnapshot, InternalError> {
    validate_snapshot(authority, snapshot, FleetSubnetRootStatus::Active)?;
    if snapshot.version != request.expected_registry {
        return Err(InternalError::conflict(
            "Coordinator snapshot differs from the host-expected active Registry version",
        ));
    }
    if request.previous_registry.authority != request.expected_registry.authority
        || request.previous_registry.revision.checked_add(1)
            != Some(request.expected_registry.revision)
    {
        return Err(InternalError::invalid_input(
            "active Registry request does not name one exact preceding revision",
        ));
    }
    let topology = ConfigOps::component_topology()?;
    let directory = FleetRegistryOps::active_directory_for_root(
        &authority.binding.authority,
        &topology,
        &snapshot.registry,
        root,
    )?;
    if directory != request.expected_directory {
        return Err(InternalError::conflict(
            "derived Fleet Directory differs from host-expected active authority",
        ));
    }
    Ok(directory)
}

fn active_response(
    root: candid::Principal,
    request: &FleetSubnetRootRegistryMirrorActivationRequest,
    active: &RootFleetRegistryActiveView,
    snapshot: &FleetRegistrySnapshotResponse,
    directory: &FleetDirectorySnapshot,
) -> Result<FleetSubnetRootRegistryMirrorActivationResponse, InternalError> {
    if active.previous_registry != request.previous_registry
        || &active.snapshot != snapshot
        || &active.directory != directory
    {
        return Err(InternalError::conflict(
            "root already contains different active Registry mirror authority",
        ));
    }
    Ok(activation_response(root, request, directory.clone()))
}

fn activation_response(
    root: candid::Principal,
    request: &FleetSubnetRootRegistryMirrorActivationRequest,
    directory: FleetDirectorySnapshot,
) -> FleetSubnetRootRegistryMirrorActivationResponse {
    FleetSubnetRootRegistryMirrorActivationResponse {
        fleet_subnet_root: root,
        previous_registry: request.previous_registry.clone(),
        version: request.expected_registry.clone(),
        directory,
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
