//! Module: workflow::component_registry
//!
//! Responsibility: prepare and verify one root's empty Component Registry authority.
//! Does not own: Component identity allocation, Canister creation, or runtime activation.
//! Boundary: preparation follows exact Store and active Registry Mirror/Directory verification.

use crate::{
    ops::{
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
    },
    view::component_registry::RootComponentRegistryView,
    workflow::bootstrap::root_store,
};
use canic_core::{
    api::fleet_activation::FleetActivationApi,
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::{config::ConfigOps, fleet_registry::FleetRegistryOps, ic::IcOps},
    },
    dto::{
        component_registry::{
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
        },
        fleet_registry::{FleetSubnetRootEntry, FleetSubnetRootStatus},
    },
};

/// Prepare the one empty Component Registry meta record under exact active root authority.
pub async fn prepare(
    request: RootComponentRegistryPreparationRequest,
) -> Result<RootComponentRegistryStatusResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    validate_active_authority(&authority, root, &request)?;

    let prepared = ComponentRegistryOps::prepare(
        authority.binding,
        request.expected_fleet_registry,
        authority.initial_release_set,
    )?;
    response(root, &prepared)
}

/// Independently verify the durable Component Registry meta record without mutation.
pub async fn status(
    request: RootComponentRegistryPreparationRequest,
) -> Result<RootComponentRegistryStatusResponse, InternalError> {
    let (authority, root) = root_authority()?;
    root_store::status(request.store_bootstrap.clone()).await?;
    validate_active_authority(&authority, root, &request)?;

    let prepared = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    if prepared.root != authority.binding
        || prepared.prepared_against_registry != request.expected_fleet_registry
        || prepared.release_set != authority.initial_release_set
    {
        return Err(InternalError::conflict(
            "durable Component Registry authority differs from the active root",
        ));
    }
    response(root, &prepared)
}

fn validate_active_authority(
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
    root: candid::Principal,
    request: &RootComponentRegistryPreparationRequest,
) -> Result<(), InternalError> {
    let active = FleetRegistryMirrorOps::current().active.ok_or_else(|| {
        InternalError::unavailable("root has no active Fleet Registry Mirror and Directory")
    })?;
    if active.snapshot.version != request.expected_fleet_registry {
        return Err(InternalError::conflict(
            "active root Registry Mirror differs from Component Registry preparation authority",
        ));
    }

    let topology = ConfigOps::component_topology()?;
    FleetRegistryOps::validate(
        &authority.binding.authority,
        &topology,
        &active.snapshot.registry,
    )?;
    let manifest = FleetRegistryOps::manifest(
        &authority.binding.authority,
        &topology,
        &active.snapshot.registry,
    )?;
    let version = FleetRegistryOps::version(
        &authority.binding.authority,
        &topology,
        &active.snapshot.registry,
    )?;
    let expected_entry = FleetSubnetRootEntry {
        placement_subnet: authority.binding.placement_subnet,
        fleet_subnet_root: root,
        component_admissions: authority.binding.component_admissions.clone(),
        component_topology_digest: authority.binding.component_topology_digest,
        active_release_set: authority.initial_release_set,
        limits: authority.binding.limits.clone(),
        status: FleetSubnetRootStatus::Active,
    };
    let directory = FleetRegistryOps::active_directory_for_root(
        &authority.binding.authority,
        &topology,
        &active.snapshot.registry,
        root,
    )?;
    if active.snapshot.manifest != manifest
        || active.snapshot.version != version
        || !active
            .snapshot
            .registry
            .fleet_subnet_roots
            .iter()
            .any(|entry| entry == &expected_entry)
        || active.directory != directory
    {
        return Err(InternalError::invalid_input(
            "active root Registry Mirror or Fleet Directory differs from protected authority",
        ));
    }
    Ok(())
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
    prepared: &RootComponentRegistryView,
) -> Result<RootComponentRegistryStatusResponse, InternalError> {
    if prepared.root.fleet_subnet_root != root {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component Registry authority does not name this root",
        ));
    }
    Ok(RootComponentRegistryStatusResponse {
        fleet_subnet_root: root,
        prepared_against_registry: prepared.prepared_against_registry.clone(),
        release_set: prepared.release_set,
        component_topology_digest: prepared.root.component_topology_digest,
        next_allocation_sequence: prepared.next_allocation_sequence,
        reserved_component_instances: prepared.reserved_component_instances,
        committed_component_instances: prepared.committed_component_instances,
        managed_descendants: prepared.managed_descendants,
        encoded_bytes: prepared.encoded_bytes,
    })
}
