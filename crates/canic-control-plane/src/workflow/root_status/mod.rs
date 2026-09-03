//! Module: workflow::root_status
//!
//! Responsibility: resolve and authorize one Root-local durable operation across existing owners.
//! Does not own: operation persistence, generic operation state, or autonomous advancement.
//! Boundary: domain records retain identity and access authority; this workflow only selects one response.

use crate::{
    dto::root::{
        RootComponentChildOperationStatus, RootComponentRemovalOperationStatus,
        RootOperationStatusResponse,
    },
    ops::{
        component_provisioning::{RootComponentProvisioningOps, status_response},
        storage::state::root_wasm_store::RootWasmStoreStateOps,
    },
    workflow::{
        component_provisioning, component_registry, fleet_registry_mirror, fleet_subnet_root,
        root_authority::validated_root_authority,
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError, ops::icp_refill::IcpRefillStoreOps,
        workflow::runtime::fleet_activation::FleetActivationWorkflow,
    },
    dto::{
        component_provisioning::RootComponentProvisioningStatusResponse,
        component_registry::RootComponentChildAllocationResponse,
    },
};

#[derive(Clone, Copy)]
enum RootOperationObserver {
    Controller,
    CoordinatorOrController,
    Preauthorized,
}

struct RootOperationMatch {
    observer: RootOperationObserver,
    status: RootOperationStatusResponse,
}

impl RootOperationMatch {
    const fn new(status: RootOperationStatusResponse, observer: RootOperationObserver) -> Self {
        Self { observer, status }
    }
}

/// Resolve the exact Root Component-provisioning owner without passing through
/// the intentionally heterogeneous operation catalogue.
pub fn component_provisioning_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }
    require_coordinator_or_controller(caller, caller_is_controller)?;
    let current = RootComponentProvisioningOps::status_by_operation_id(operation_id)?
        .ok_or_else(InternalError::unavailable)?;
    component_provisioning::require_current_claim_capacity(&current)?;
    Ok(status_response(current))
}

/// Resolve one exact Root child-allocation owner without passing through the
/// intentionally heterogeneous operation catalogue.
pub fn component_child_provisioning_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }
    component_registry::child_allocation_operation_status(
        operation_id,
        caller,
        caller_is_controller,
    )?
    .ok_or_else(InternalError::unavailable)
}

/// Resolve an operation identity only through domain owners that already have an exact ID index.
pub fn operation_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<RootOperationStatusResponse, InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }

    let mut matches = Vec::new();
    if let Some(adoption) = fleet_subnet_root::wasm_store_adoption_operation_status(operation_id)? {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::AdoptStore(adoption),
            RootOperationObserver::Controller,
        ));
    }
    if let Some(bootstrap) =
        RootWasmStoreStateOps::root_store_bootstrap_receipt_by_operation(operation_id)
    {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::BootstrapStore(bootstrap),
            RootOperationObserver::Controller,
        ));
    }
    let activation = FleetActivationWorkflow::status()?;
    if activation.identity.operation_id == operation_id {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::FleetActivation(activation),
            RootOperationObserver::Controller,
        ));
    }
    if let Some(allocation) = component_registry::child_allocation_operation_status(
        operation_id,
        caller,
        caller_is_controller,
    )? {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::ProvisionChild(RootComponentChildOperationStatus {
                allocation,
            }),
            RootOperationObserver::Preauthorized,
        ));
    }
    if let Some(allocation) =
        component_registry::allocation_operation_status(operation_id, caller, caller_is_controller)?
    {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::ProvisionComponent(allocation),
            RootOperationObserver::Preauthorized,
        ));
    }
    if let Some(provisioning) = RootComponentProvisioningOps::status_by_operation_id(operation_id)?
    {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::ProvisionComponents(status_response(provisioning)),
            RootOperationObserver::CoordinatorOrController,
        ));
    }
    if let Some(refill) = IcpRefillStoreOps::find_by_operation_id(operation_id)? {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::RefillCycles(IcpRefillStoreOps::to_response(&refill)),
            RootOperationObserver::Controller,
        ));
    }
    if let Some((draining, deletion)) =
        component_registry::component_removal_operation_status(operation_id)?
    {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::RemoveComponent(RootComponentRemovalOperationStatus {
                draining,
                deletion,
            }),
            RootOperationObserver::Controller,
        ));
    }
    if let Some(removal) = fleet_subnet_root::removal_operation_status(operation_id)? {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::RemoveRoot(removal),
            RootOperationObserver::CoordinatorOrController,
        ));
    }
    if let Some(removal) = component_registry::subtree_removal_operation_status(operation_id)? {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::RemoveSubtree(removal),
            RootOperationObserver::Controller,
        ));
    }
    if let Some(synchronization) =
        fleet_registry_mirror::synchronization_operation_status(operation_id)?
    {
        matches.push(RootOperationMatch::new(
            RootOperationStatusResponse::SynchronizeRegistry(synchronization),
            RootOperationObserver::Controller,
        ));
    }

    let selected = select_unique_match(matches)?;
    authorize_observer(selected.observer, caller, caller_is_controller)?;
    if matches!(
        &selected.status,
        RootOperationStatusResponse::ProvisionComponents(_)
    ) {
        let current = RootComponentProvisioningOps::status_by_operation_id(operation_id)?
            .ok_or_else(InternalError::invariant)?;
        component_provisioning::require_current_claim_capacity(&current)?;
    }
    Ok(selected.status)
}

const fn require_controller(caller_is_controller: bool) -> Result<(), InternalError> {
    if caller_is_controller {
        Ok(())
    } else {
        Err(InternalError::forbidden())
    }
}

fn authorize_observer(
    observer: RootOperationObserver,
    caller: Principal,
    caller_is_controller: bool,
) -> Result<(), InternalError> {
    match observer {
        RootOperationObserver::Controller => require_controller(caller_is_controller),
        RootOperationObserver::CoordinatorOrController => {
            require_coordinator_or_controller(caller, caller_is_controller)
        }
        RootOperationObserver::Preauthorized => Ok(()),
    }
}

fn require_coordinator_or_controller(
    caller: Principal,
    caller_is_controller: bool,
) -> Result<(), InternalError> {
    if caller_is_controller {
        return Ok(());
    }
    let (authority, _) = validated_root_authority()?;
    if caller == authority.binding.authority.binding.coordinator {
        Ok(())
    } else {
        Err(InternalError::forbidden())
    }
}

fn select_unique_match<T>(matches: Vec<T>) -> Result<T, InternalError> {
    let mut matches = matches.into_iter();
    let selected = matches.next().ok_or_else(InternalError::unavailable)?;
    if matches.next().is_some() {
        return Err(InternalError::invariant());
    }
    Ok(selected)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_operation_status_rejects_the_zero_identity_before_state_access() {
        let Err(error) = operation_status([0; 32], Principal::anonymous(), false) else {
            panic!("zero operation ID must fail");
        };
        assert_eq!(
            error.public_error().code(),
            canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
        );

        let Err(error) = component_provisioning_status([0; 32], Principal::anonymous(), false)
        else {
            panic!("zero Component-provisioning operation ID must fail");
        };
        assert_eq!(
            error.public_error().code(),
            canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
        );

        let Err(error) =
            component_child_provisioning_status([0; 32], Principal::anonymous(), false)
        else {
            panic!("zero child-provisioning operation ID must fail");
        };
        assert_eq!(
            error.public_error().code(),
            canic_core::diagnostics::codes::REQUEST_INVALID.raw_code()
        );
    }

    #[test]
    fn duplicate_operation_ownership_is_an_invariant_before_authorization() {
        let Err(error) = select_unique_match(vec![
            RootOperationObserver::Controller,
            RootOperationObserver::CoordinatorOrController,
        ]) else {
            panic!("duplicate operation ownership must fail");
        };
        assert_eq!(
            error.public_error().code(),
            canic_core::diagnostics::codes::STATE_INVALID.raw_code()
        );
    }
}
