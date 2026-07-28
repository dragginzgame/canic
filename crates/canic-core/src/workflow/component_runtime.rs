//! Module: workflow::component_runtime
//!
//! Responsibility: validate exact Directory authority and activate one managed Component runtime.
//! Does not own: root distribution, Registry mutation, or endpoint authorization.
//! Boundary: only exact protected binding and Directory evidence may cross each runtime phase.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        component_registry::{
            ComponentRuntimeActivationRequest, ComponentRuntimeDirectoryAuthority,
            ComponentRuntimeDirectoryPreparationRequest,
            ComponentRuntimeDirectorySynchronizationRequest, ComponentRuntimePhase,
            ComponentRuntimeStatusResponse,
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
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let current = FleetActivationOps::component_runtime_status().map_err(StorageOpsError::from)?;
    validate_request(&current, &request)?;
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&request.authority)?;
    FleetActivationOps::prepare_component_runtime_directory(request, authority_hash)
        .map_err(StorageOpsError::from)
        .map_err(InternalError::from)
}

/// Synchronize one exact next current Directory authority on an Active Component runtime.
pub fn synchronize_directory(
    request: ComponentRuntimeDirectorySynchronizationRequest,
) -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let current = status()?;
    if request.operation_id != current.operation_id {
        return Err(InternalError::conflict(
            "Component runtime Directory operation differs from protected installation identity",
        ));
    }
    validate_binding(&current.binding)?;
    validate_authority(&current.binding, &request.authority)?;
    if current.phase != ComponentRuntimePhase::Active || current.activation.is_none() {
        return Err(InternalError::conflict(
            "current Component Directory synchronization requires an Active runtime",
        ));
    }
    let current_authority = current.authority.as_ref().ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Storage,
            "Active Component runtime has no current Directory authority",
        )
    })?;
    validate_directory_progression(current_authority, &request.authority)?;
    let authority_hash = ComponentRuntimeOps::directory_authority_hash(&request.authority)?;
    FleetActivationOps::synchronize_component_runtime_directory(request, authority_hash)
        .map_err(StorageOpsError::from)
        .map_err(InternalError::from)
}

/// Independently validate and return the target-local Directory preparation state.
pub fn status() -> Result<ComponentRuntimeStatusResponse, InternalError> {
    let status = FleetActivationOps::component_runtime_status().map_err(StorageOpsError::from)?;
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

/// Activate one exact Directory-prepared Component runtime.
pub fn activate(
    request: ComponentRuntimeActivationRequest,
) -> Result<crate::view::fleet_activation::ComponentRuntimeActivationTransition, InternalError> {
    let current = status()?;
    let expected_authority_hash = match current.phase {
        ComponentRuntimePhase::AwaitingDirectory | ComponentRuntimePhase::DirectoryPrepared => {
            current.authority_hash
        }
        ComponentRuntimePhase::Active => current
            .activation
            .map(|activation| activation.directory_authority_hash),
    };
    if request.operation_id != current.operation_id
        || request.directory_authority_hash == [0; 32]
        || expected_authority_hash != Some(request.directory_authority_hash)
    {
        return Err(InternalError::conflict(
            "Component runtime activation differs from its protected Directory authority",
        ));
    }
    let transition = FleetActivationOps::activate_component_runtime(request, IcOps::now_nanos())
        .map_err(StorageOpsError::from)
        .map_err(InternalError::from)?;
    if transition.transitioned
        && let Err(error) = crate::workflow::runtime::RuntimeWorkflow::start_all()
    {
        IcOps::trap(format!(
            "Component runtime activation could not establish runtime services: {error}"
        ));
    }
    Ok(transition)
}

fn validate_request(
    current: &ComponentRuntimeStatusResponse,
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

fn validate_directory_progression(
    current: &ComponentRuntimeDirectoryAuthority,
    next: &ComponentRuntimeDirectoryAuthority,
) -> Result<(), InternalError> {
    let current_component = &current.component.provenance;
    let next_component = &next.component.provenance;
    let expected_revision = current_component
        .component_registry_revision
        .checked_add(1)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry revision overflow"))?;
    let current_fleet_revision = current.fleet.provenance.registry.revision;
    let next_fleet_revision = next.fleet.provenance.registry.revision;
    if next_component.component != current_component.component
        || next_component.source_fleet_subnet_root != current_component.source_fleet_subnet_root
        || next_component.component_registry_revision != expected_revision
        || next_component.component_registry_content_hash
            == current_component.component_registry_content_hash
        || next_component.synchronized_at_ns <= current_component.synchronized_at_ns
        || next_fleet_revision < current_fleet_revision
        || (next_fleet_revision == current_fleet_revision && next.fleet != current.fleet)
    {
        return Err(InternalError::conflict(
            "next Component Directory does not advance exact current authority",
        ));
    }
    Ok(())
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
