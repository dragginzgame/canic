//! Module: workflow::component_registry
//!
//! Responsibility: prepare Component Registry authority and reserve top-level identities.
//! Does not own: Canister creation, installation, Component Directories, or runtime activation.
//! Boundary: every mutation follows exact Store and active Registry Mirror/Directory verification.

use crate::{
    ops::{
        component_registry::ComponentRegistryOps, fleet_registry_mirror::FleetRegistryMirrorOps,
    },
    view::component_registry::{RootComponentAllocationView, RootComponentRegistryView},
    workflow::bootstrap::root_store,
};
use canic_core::{
    api::fleet_activation::FleetActivationApi,
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::{config::ConfigOps, fleet_registry::FleetRegistryOps, ic::IcOps},
        policy::component_allocation::{
            TopLevelComponentAllocationInput, reserve_top_level_component,
        },
    },
    dto::{
        component_registry::{
            ComponentProvisioningOrigin, RootComponentAllocationPhase,
            RootComponentAllocationRequest, RootComponentAllocationResponse,
            RootComponentAllocationStatusRequest, RootComponentRegistryPreparationRequest,
            RootComponentRegistryStatusResponse,
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
        request.store_bootstrap,
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
        || prepared.store_bootstrap != request.store_bootstrap
    {
        return Err(InternalError::conflict(
            "durable Component Registry authority differs from the active root",
        ));
    }
    response(root, &prepared)
}

/// Durably reserve one admitted top-level Component identity and root-local capacity.
pub async fn reserve_allocation(
    request: RootComponentAllocationRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let (authority, root) = root_authority()?;
    let prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: prepared.store_bootstrap.clone(),
        expected_fleet_registry: prepared.prepared_against_registry.clone(),
    };
    root_store::status(preparation_request.store_bootstrap.clone()).await?;
    validate_active_authority(&authority, root, &preparation_request)?;

    let provisioning_origin = ComponentProvisioningOrigin::FleetAdministrator {
        caller: IcOps::msg_caller(),
    };
    let topology = ConfigOps::component_topology()?;
    if let Some(existing) = ComponentRegistryOps::allocation(request.operation_id) {
        if existing.component_spec != request.component_spec
            || existing.provisioning_origin != provisioning_origin
        {
            return Err(InternalError::conflict(
                "Component allocation operation is already bound to different intent",
            ));
        }
        validate_allocation_record(
            &authority.binding,
            authority.initial_release_set,
            &topology,
            &existing,
            request.operation_id,
        )?;
        return allocation_response(existing);
    }

    let counts = ComponentRegistryOps::component_spec_counts(&request.component_spec)?;
    let decision = reserve_top_level_component(TopLevelComponentAllocationInput {
        operation_id: request.operation_id,
        component_spec: &request.component_spec,
        root: &authority.binding,
        topology: &topology,
        next_allocation_sequence: prepared.next_allocation_sequence,
        reserved_component_instances: prepared.reserved_component_instances,
        committed_component_instances: prepared.committed_component_instances,
        managed_descendants: prepared.managed_descendants,
        reserved_spec_instances: counts.reserved,
        committed_spec_instances: counts.committed,
    })
    .map_err(InternalError::from)?;
    let reserved = ComponentRegistryOps::reserve_allocation(
        decision,
        request.operation_id,
        provisioning_origin,
    )?;
    allocation_response(reserved)
}

/// Read one durable top-level Component allocation reservation without mutation.
pub fn allocation_status(
    request: RootComponentAllocationStatusRequest,
) -> Result<RootComponentAllocationResponse, InternalError> {
    let (authority, _root) = root_authority()?;
    let _prepared = prepared_registry(&authority.binding, authority.initial_release_set)?;
    let allocation = ComponentRegistryOps::allocation(request.operation_id).ok_or_else(|| {
        InternalError::unavailable("Component allocation operation has not been reserved")
    })?;
    let topology = ConfigOps::component_topology()?;
    validate_allocation_record(
        &authority.binding,
        authority.initial_release_set,
        &topology,
        &allocation,
        request.operation_id,
    )?;
    allocation_response(allocation)
}

fn prepared_registry(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
) -> Result<RootComponentRegistryView, InternalError> {
    let prepared = ComponentRegistryOps::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    if &prepared.root != root || prepared.release_set != release_set {
        return Err(InternalError::conflict(
            "durable Component Registry authority differs from the protected root",
        ));
    }
    Ok(prepared)
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

fn allocation_response(
    allocation: RootComponentAllocationView,
) -> Result<RootComponentAllocationResponse, InternalError> {
    if allocation.allocation_sequence == 0 {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation sequence is zero",
        ));
    }
    Ok(RootComponentAllocationResponse {
        operation_id: allocation.operation_id,
        allocation_sequence: allocation.allocation_sequence,
        component: allocation.component,
        component_spec: allocation.component_spec,
        spec_hash: allocation.spec_hash,
        role: allocation.role,
        provisioning_origin: allocation.provisioning_origin,
        release_set: allocation.release_set,
        phase: RootComponentAllocationPhase::Reserved,
    })
}

fn validate_allocation_record(
    root: &canic_core::ids::FleetSubnetRootBinding,
    release_set: canic_core::ids::FleetSubnetRootReleaseSet,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    allocation: &RootComponentAllocationView,
    expected_operation_id: [u8; 32],
) -> Result<(), InternalError> {
    if allocation.operation_id == [0; 32] || allocation.operation_id != expected_operation_id {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation operation identity is invalid",
        ));
    }
    if allocation.allocation_sequence == 0
        || allocation.component
            != canic_core::ids::ComponentInstanceId::from_root_allocation(
                root.authority.binding.fleet.fleet,
                root.authority.epoch,
                root.fleet_subnet_root,
                allocation.allocation_sequence,
            )
    {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation identity differs from its root-local sequence",
        ));
    }
    if allocation.release_set != release_set {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation release set differs from protected root authority",
        ));
    }
    let admission = root
        .component_admissions
        .binary_search_by(|candidate| candidate.component_spec.cmp(&allocation.component_spec))
        .ok()
        .map(|index| &root.component_admissions[index])
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Storage,
                "stored Component allocation Spec is not admitted by its protected root",
            )
        })?;
    let spec = topology.get(&allocation.component_spec).ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation Spec is absent from the protected topology",
        )
    })?;
    if allocation.spec_hash != admission.spec_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation hash differs from its protected root admission",
        ));
    }
    if allocation.spec_hash != spec.spec_hash {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation hash differs from its protected Spec",
        ));
    }
    if allocation.role != spec.component_role {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "stored Component allocation role differs from its protected Spec",
        ));
    }
    Ok(())
}
