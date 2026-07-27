//! Module: workflow::fleet_registry_mirror
//!
//! Responsibility: fetch, validate, durably stage, and acknowledge one root's Registry snapshot.
//! Does not own: Coordinator Registry mutation, Fleet Directory activation, or runtime activation.
//! Boundary: acknowledgement is sent only after exact root, topology, Store, and snapshot evidence.

use crate::{
    storage::stable::fleet_registry_mirror::{
        RootFleetRegistryCandidateRecord, RootFleetRegistryMirrorStore,
    },
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
            FleetRegistrySnapshotResponse, FleetSubnetRootEntry,
            FleetSubnetRootRegistrySyncRequest, FleetSubnetRootRegistrySyncResponse,
            FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
            FleetSubnetRootStatus,
        },
    },
    protocol,
};

/// Fetch and acknowledge the current all-Joining snapshot.
pub async fn synchronize(
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<FleetSubnetRootRegistrySyncResponse, InternalError> {
    let authority = FleetActivationApi::root_authority().map_err(InternalError::public)?;
    let root = IcOps::canister_self();
    if authority.binding.fleet_subnet_root != root {
        return Err(InternalError::invalid_input(
            "protected Fleet Subnet Root authority does not name this Canister",
        ));
    }
    root_store::status(request.store_bootstrap.clone()).await?;

    let snapshot = fetch_snapshot(authority.binding.authority.binding.coordinator).await?;
    validate_snapshot(&authority, &snapshot)?;
    if snapshot.version != request.expected_registry {
        return Err(InternalError::conflict(
            "Coordinator snapshot differs from the host-expected Registry version",
        ));
    }
    let candidate = RootFleetRegistryMirrorStore::export().candidate;
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
        RootFleetRegistryMirrorStore::commit(RootFleetRegistryCandidateRecord {
            snapshot: snapshot.clone(),
            acknowledgement: None,
        });
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
    RootFleetRegistryMirrorStore::commit(RootFleetRegistryCandidateRecord {
        snapshot: snapshot.clone(),
        acknowledgement: Some(acknowledgement.clone()),
    });
    response(root, &snapshot, acknowledgement)
}

/// Revalidate durable local evidence without changing it.
pub async fn status(
    request: FleetSubnetRootRegistrySyncRequest,
) -> Result<FleetSubnetRootRegistrySyncResponse, InternalError> {
    let authority = FleetActivationApi::root_authority().map_err(InternalError::public)?;
    let root = IcOps::canister_self();
    root_store::status(request.store_bootstrap).await?;
    let candidate = RootFleetRegistryMirrorStore::export()
        .candidate
        .ok_or_else(|| InternalError::unavailable("root has no staged Fleet Registry snapshot"))?;
    validate_snapshot(&authority, &candidate.snapshot)?;
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
        status: FleetSubnetRootStatus::Joining,
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
