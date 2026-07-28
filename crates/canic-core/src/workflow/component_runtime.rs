//! Module: workflow::component_runtime
//!
//! Responsibility: validate and prepare exact Directory authority for one managed Component node.
//! Does not own: root distribution, Registry mutation, endpoint authorization, or activation.
//! Boundary: only the protected root may reach the endpoint; retained authority is revalidated.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        component_registry::{
            ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimeDirectoryStatusResponse,
        },
        fleet_registry::{FleetDirectorySnapshot, FleetSubnetRootStatus},
    },
    ids::{ComponentBinding, ManagedCanisterBinding},
    ops::{
        component_runtime::ComponentRuntimeOps,
        ic::IcOps,
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
};

/// Prepare one exact Fleet and Component Directory authority while runtime remains Prepared.
pub fn prepare_directory(
    request: ComponentRuntimeDirectoryPreparationRequest,
) -> Result<ComponentRuntimeDirectoryStatusResponse, InternalError> {
    let current =
        FleetActivationOps::component_runtime_directory_status().map_err(StorageOpsError::from)?;
    validate_request(&current, &request)?;
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&request.authority)?;
    FleetActivationOps::prepare_component_runtime_directory(request, authority_hash)
        .map_err(StorageOpsError::from)
        .map_err(InternalError::from)
}

/// Independently validate and return the target-local Directory preparation state.
pub fn directory_status() -> Result<ComponentRuntimeDirectoryStatusResponse, InternalError> {
    let status =
        FleetActivationOps::component_runtime_directory_status().map_err(StorageOpsError::from)?;
    validate_binding(&status.binding)?;
    match (&status.authority, status.authority_hash) {
        (None, None) => {}
        (Some(authority), Some(authority_hash)) => {
            validate_authority(&status.binding, authority)?;
            if ComponentRuntimeOps::directory_authority_hash(authority)? != authority_hash {
                return Err(InternalError::invariant(
                    InternalErrorOrigin::Storage,
                    "retained Component runtime Directory authority hash does not match its value",
                ));
            }
        }
        _ => {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "retained Component runtime Directory authority and hash are incomplete",
            ));
        }
    }
    Ok(status)
}

fn validate_request(
    current: &ComponentRuntimeDirectoryStatusResponse,
    request: &ComponentRuntimeDirectoryPreparationRequest,
) -> Result<(), InternalError> {
    if request.operation_id != current.operation_id {
        return Err(InternalError::conflict(
            "Component runtime Directory operation differs from protected installation identity",
        ));
    }
    validate_binding(&current.binding)?;
    validate_authority(&current.binding, &request.authority)
}

fn validate_binding(binding: &ManagedCanisterBinding) -> Result<(), InternalError> {
    let canister = match binding {
        ManagedCanisterBinding::Component(component) => component.canister_id,
        ManagedCanisterBinding::ComponentChild(child) => child.canister_id,
    };
    if canister != IcOps::canister_self() {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Storage,
            "protected Component runtime binding does not name this Canister",
        ));
    }
    Ok(())
}

fn validate_authority(
    binding: &ManagedCanisterBinding,
    authority: &ComponentRuntimeDirectoryAuthority,
) -> Result<(), InternalError> {
    let component = owning_component(binding);
    if authority.component.provenance.component != *component
        || authority.component.provenance.source_fleet_subnet_root != component.fleet_subnet_root
        || authority.component.provenance.component_registry_revision == 0
        || authority
            .component
            .provenance
            .component_registry_content_hash
            == [0; 32]
        || authority.component.provenance.synchronized_at_ns == 0
    {
        return Err(InternalError::invalid_input(
            "Component Directory does not match the protected Component-tree binding",
        ));
    }
    validate_fleet_directory(component, &authority.fleet)
}

fn validate_fleet_directory(
    component: &ComponentBinding,
    directory: &FleetDirectorySnapshot,
) -> Result<(), InternalError> {
    if directory.provenance.registry.authority != component.authority
        || directory.provenance.registry.revision == 0
        || directory.provenance.registry.content_hash == [0; 32]
        || directory.provenance.source_fleet_subnet_root != component.fleet_subnet_root
        || directory.fleet_subnet_roots.is_empty()
    {
        return Err(InternalError::invalid_input(
            "Fleet Directory does not match the protected Component-tree binding",
        ));
    }

    let mut previous: Option<(&[u8], &[u8])> = None;
    let mut found_source = false;
    for entry in &directory.fleet_subnet_roots {
        let key = (
            entry.placement_subnet.as_principal().as_slice(),
            entry.fleet_subnet_root.as_slice(),
        );
        if entry.status != FleetSubnetRootStatus::Active
            || previous.is_some_and(|previous| previous >= key)
        {
            return Err(InternalError::invalid_input(
                "Fleet Directory root entries are not unique, canonical and Active",
            ));
        }
        previous = Some(key);
        if entry.fleet_subnet_root == component.fleet_subnet_root {
            if found_source || entry.placement_subnet != component.placement_subnet {
                return Err(InternalError::invalid_input(
                    "Fleet Directory source root placement differs from the Component binding",
                ));
            }
            found_source = true;
        }
    }
    if !found_source {
        return Err(InternalError::invalid_input(
            "Fleet Directory omits the Component's source root",
        ));
    }
    Ok(())
}

const fn owning_component(binding: &ManagedCanisterBinding) -> &ComponentBinding {
    match binding {
        ManagedCanisterBinding::Component(component) => component,
        ManagedCanisterBinding::ComponentChild(child) => &child.component,
    }
}
