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
        component_registry, fleet_registry_mirror, fleet_subnet_root,
        root_authority::validated_root_authority,
    },
};
use candid::Principal;
use canic_core::control_plane_support::{
    error::InternalError, ops::icp_refill::IcpRefillStoreOps,
    workflow::runtime::fleet_activation::FleetActivationWorkflow,
};

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
        require_controller(caller_is_controller)?;
        matches.push(RootOperationStatusResponse::AdoptStore(adoption));
    }
    if let Some(bootstrap) =
        RootWasmStoreStateOps::root_store_bootstrap_receipt_by_operation(operation_id)
    {
        require_controller(caller_is_controller)?;
        matches.push(RootOperationStatusResponse::BootstrapStore(bootstrap));
    }
    let activation = FleetActivationWorkflow::status()?;
    if activation.identity.operation_id == operation_id {
        require_controller(caller_is_controller)?;
        matches.push(RootOperationStatusResponse::FleetActivation(activation));
    }
    if let Some(allocation) = component_registry::child_allocation_operation_status(
        operation_id,
        caller,
        caller_is_controller,
    )? {
        matches.push(RootOperationStatusResponse::ProvisionChild(
            RootComponentChildOperationStatus { allocation },
        ));
    }
    if let Some(allocation) =
        component_registry::allocation_operation_status(operation_id, caller, caller_is_controller)?
    {
        matches.push(RootOperationStatusResponse::ProvisionComponent(allocation));
    }
    if let Some(provisioning) = RootComponentProvisioningOps::status_by_operation_id(operation_id)?
    {
        if !caller_is_controller {
            let (authority, _root) = validated_root_authority()?;
            if caller != authority.binding.authority.binding.coordinator {
                return Err(InternalError::forbidden());
            }
        }
        matches.push(RootOperationStatusResponse::ProvisionComponents(
            status_response(provisioning),
        ));
    }
    if let Some(refill) = IcpRefillStoreOps::find_by_operation_id(operation_id)? {
        require_controller(caller_is_controller)?;
        matches.push(RootOperationStatusResponse::RefillCycles(
            IcpRefillStoreOps::to_response(&refill),
        ));
    }
    if let Some((draining, deletion)) =
        component_registry::component_removal_operation_status(operation_id)?
    {
        require_controller(caller_is_controller)?;
        matches.push(RootOperationStatusResponse::RemoveComponent(
            RootComponentRemovalOperationStatus { draining, deletion },
        ));
    }
    if let Some(removal) = fleet_subnet_root::removal_operation_status(operation_id)? {
        require_root_removal_observer(caller, caller_is_controller)?;
        matches.push(RootOperationStatusResponse::RemoveRoot(removal));
    }
    if let Some(removal) = component_registry::subtree_removal_operation_status(operation_id)? {
        require_controller(caller_is_controller)?;
        matches.push(RootOperationStatusResponse::RemoveSubtree(removal));
    }
    if let Some(synchronization) =
        fleet_registry_mirror::synchronization_operation_status(operation_id)?
    {
        require_controller(caller_is_controller)?;
        matches.push(RootOperationStatusResponse::SynchronizeRegistry(
            synchronization,
        ));
    }

    let mut matches = matches.into_iter();
    let status = matches.next().ok_or_else(InternalError::unavailable)?;
    if matches.next().is_some() {
        return Err(InternalError::invariant());
    }
    Ok(status)
}

const fn require_controller(caller_is_controller: bool) -> Result<(), InternalError> {
    if caller_is_controller {
        Ok(())
    } else {
        Err(InternalError::forbidden())
    }
}

fn require_root_removal_observer(
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
    }
}
