//! Module: workflow::root_status
//!
//! Responsibility: resolve and authorize one Root-local durable operation across existing owners.
//! Does not own: operation persistence, generic operation state, or autonomous advancement.
//! Boundary: domain records retain identity and access authority; reads enforce unique ownership.

use crate::{
    dto::root::RootComponentRemovalOperationStatus,
    ops::{
        component_provisioning::{RootComponentProvisioningOps, status_response},
        storage::state::root_wasm_store::RootWasmStoreStateOps,
    },
    workflow::{
        component_provisioning, component_registry, fleet_registry_mirror, fleet_subnet_root,
        root_authority::validated_root_authority,
    },
};
use canic_contracts::dto::{
    component_provisioning::RootComponentProvisioningStatusResponse,
    component_registry::RootComponentChildAllocationResponse,
};
use canic_core::control_plane_support::{
    error::InternalError, ops::icp_refill::IcpRefillStoreOps,
    workflow::runtime::fleet_activation::FleetActivationWorkflow,
};
use ic_platform::principal::ExternalPrincipal as Principal;

#[derive(Clone, Copy)]
enum RootOperationObserver {
    Controller,
    CoordinatorOrController,
    Preauthorized,
}

/// Read exact Root provisioning evidence after ownership and observer checks.
pub fn component_provisioning_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<RootComponentProvisioningStatusResponse, InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }
    resolve_operation(operation_id, caller, caller_is_controller)?;
    let current = RootComponentProvisioningOps::status_by_operation_id(operation_id)?
        .ok_or_else(InternalError::unavailable)?;
    component_provisioning::require_current_claim_capacity(&current)?;
    Ok(status_response(current))
}

/// Read exact child-allocation evidence after ownership and registered-parent checks.
pub fn component_child_provisioning_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<RootComponentChildAllocationResponse, InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }
    resolve_operation(operation_id, caller, caller_is_controller)?;
    component_registry::child_allocation_operation_status(
        operation_id,
        caller,
        caller_is_controller,
    )?
    .ok_or_else(InternalError::unavailable)
}

/// Read top-level allocation evidence without advancing its lifecycle.
pub fn component_allocation_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<canic_contracts::dto::root::RootComponentOperationStatus, InternalError> {
    resolve_operation(operation_id, caller, caller_is_controller)?;
    component_registry::allocation_operation_status(operation_id, caller, caller_is_controller)?
        .ok_or_else(InternalError::unavailable)
}

/// Observe one retained subtree operation under its original controller authority.
pub fn subtree_removal_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<
    canic_contracts::dto::component_registry::RootComponentSubtreeRemovalResponse,
    InternalError,
> {
    resolve_operation(operation_id, caller, caller_is_controller)?;
    component_registry::subtree_removal_operation_status(operation_id)?
        .ok_or_else(InternalError::unavailable)
}

/// Read retained component removal without advancing destructive effects.
pub fn component_removal_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<RootComponentRemovalOperationStatus, InternalError> {
    resolve_operation(operation_id, caller, caller_is_controller)?;
    let (draining, deletion) =
        component_registry::component_removal_operation_status(operation_id)?
            .ok_or_else(InternalError::unavailable)?;
    Ok(RootComponentRemovalOperationStatus { draining, deletion })
}

/// Resolve an operation identity only through domain owners that already have an exact ID index.
fn resolve_operation(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<(), InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::invalid_input());
    }

    let mut matches = Vec::new();
    if fleet_subnet_root::wasm_store_adoption_operation_status(operation_id)?.is_some() {
        matches.push(RootOperationObserver::Controller);
    }
    if RootWasmStoreStateOps::root_store_bootstrap_receipt_by_operation(operation_id).is_some() {
        matches.push(RootOperationObserver::Controller);
    }
    let activation = FleetActivationWorkflow::status()?;
    if activation.identity.operation_id == operation_id {
        matches.push(RootOperationObserver::Controller);
    }
    if component_registry::child_allocation_operation_status(
        operation_id,
        caller,
        caller_is_controller,
    )?
    .is_some()
    {
        matches.push(RootOperationObserver::Preauthorized);
    }
    if component_registry::allocation_operation_status(operation_id, caller, caller_is_controller)?
        .is_some()
    {
        matches.push(RootOperationObserver::Preauthorized);
    }
    if RootComponentProvisioningOps::status_by_operation_id(operation_id)?.is_some() {
        matches.push(RootOperationObserver::CoordinatorOrController);
    }
    if IcpRefillStoreOps::find_by_operation_id(operation_id)?.is_some() {
        matches.push(RootOperationObserver::Controller);
    }
    if component_registry::component_removal_operation_status(operation_id)?.is_some() {
        matches.push(RootOperationObserver::Controller);
    }
    if fleet_subnet_root::removal_operation_status(operation_id)?.is_some() {
        matches.push(RootOperationObserver::CoordinatorOrController);
    }
    if component_registry::subtree_removal_operation_status(operation_id)?.is_some() {
        matches.push(RootOperationObserver::Controller);
    }
    if fleet_registry_mirror::synchronization_operation_status(operation_id)?.is_some() {
        matches.push(RootOperationObserver::Controller);
    }

    let selected = select_unique_match(matches)?;
    authorize_observer(selected, caller, caller_is_controller)
}

/// Read original Root-removal progress after unique ownership and observer checks.
pub fn removal_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<canic_contracts::dto::root::RootRemovalOperationStatus, InternalError> {
    resolve_operation(operation_id, caller, caller_is_controller)?;
    fleet_subnet_root::removal_operation_status(operation_id)?
        .ok_or_else(InternalError::unavailable)
}

/// Retain collision and observer checks before returning exact adoption evidence.
pub fn store_adoption_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<
    canic_contracts::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse,
    InternalError,
> {
    resolve_operation(operation_id, caller, caller_is_controller)?;
    fleet_subnet_root::wasm_store_adoption_operation_status(operation_id)?
        .ok_or_else(InternalError::unavailable)
}

/// Read the original bootstrap receipt without publishing or retrying Store effects.
pub fn store_bootstrap_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<canic_contracts::dto::root_store::RootStoreBootstrapResponse, InternalError> {
    resolve_operation(operation_id, caller, caller_is_controller)?;
    RootWasmStoreStateOps::root_store_bootstrap_receipt_by_operation(operation_id)
        .ok_or_else(InternalError::unavailable)
}

/// Read only the exact retained activation identity and credential evidence.
pub fn activation_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<canic_contracts::dto::fleet_activation::FleetActivationStatusResponse, InternalError> {
    resolve_operation(operation_id, caller, caller_is_controller)?;
    let status = FleetActivationWorkflow::status()?;
    if status.identity.operation_id != operation_id {
        return Err(InternalError::unavailable());
    }
    Ok(status)
}

/// Observe the original refill; never transfer, notify or replace an absent operation.
pub fn refill_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<canic_contracts::dto::icp_refill::IcpRefillResponse, InternalError> {
    resolve_operation(operation_id, caller, caller_is_controller)?;
    let record = IcpRefillStoreOps::find_by_operation_id(operation_id)?
        .ok_or_else(InternalError::unavailable)?;
    Ok(IcpRefillStoreOps::to_response(&record))
}

/// Read retained synchronization and activation without advancing either phase.
pub fn registry_synchronization_status(
    operation_id: [u8; 32],
    caller: Principal,
    caller_is_controller: bool,
) -> Result<canic_contracts::dto::root::RootRegistrySynchronizationOperationStatus, InternalError> {
    resolve_operation(operation_id, caller, caller_is_controller)?;
    fleet_registry_mirror::synchronization_operation_status(operation_id)?
        .ok_or_else(InternalError::unavailable)
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
    fn removal_read_rejects_the_zero_identity_before_state_access() {
        let Err(error) = removal_status([0; 32], Principal::anonymous(), false) else {
            panic!("zero operation ID must fail");
        };
        assert_eq!(
            error.public_error().code(),
            canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
        );

        let Err(error) = component_provisioning_status([0; 32], Principal::anonymous(), false)
        else {
            panic!("zero Component-provisioning operation ID must fail");
        };
        assert_eq!(
            error.public_error().code(),
            canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
        );

        let Err(error) =
            component_child_provisioning_status([0; 32], Principal::anonymous(), false)
        else {
            panic!("zero child-provisioning operation ID must fail");
        };
        assert_eq!(
            error.public_error().code(),
            canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
        );
    }

    #[test]
    fn allocation_read_rejects_zero_before_accessing_state() {
        let error =
            component_allocation_status([0; 32], Principal::anonymous(), false).unwrap_err();
        assert_eq!(
            error.public_error().code(),
            canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
        );
    }

    #[test]
    fn subtree_read_rejects_zero_before_state_access() {
        let error = subtree_removal_status([0; 32], Principal::anonymous(), false).unwrap_err();
        assert_eq!(
            error.public_error().code(),
            canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
        );
    }

    #[test]
    fn component_removal_read_rejects_zero_before_state_access() {
        let error = component_removal_status([0; 32], Principal::anonymous(), false).unwrap_err();
        assert_eq!(
            error.public_error().code(),
            canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
        );
    }

    #[test]
    fn store_reads_reject_zero_identity_before_state_access() {
        for error in [
            store_adoption_status([0; 32], Principal::anonymous(), false).err(),
            store_bootstrap_status([0; 32], Principal::anonymous(), false).err(),
        ] {
            assert_eq!(
                error.unwrap().public_error().code(),
                canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
            );
        }
    }

    #[test]
    fn native_store_ownership_still_participates_in_collision_checks() {
        let matches = vec![
            RootOperationObserver::Controller,
            RootOperationObserver::Controller,
        ];
        let error = select_unique_match(matches)
            .err()
            .expect("two owners cannot share one operation ID");
        assert_eq!(
            error.public_error().code(),
            canic_contracts::diagnostics::codes::STATE_INVALID.raw_code()
        );
        assert_eq!(
            authorize_observer(
                RootOperationObserver::Controller,
                Principal::anonymous(),
                false
            )
            .unwrap_err()
            .public_error()
            .code(),
            InternalError::forbidden().public_error().code()
        );
    }

    #[test]
    fn activation_and_refill_reads_reject_zero_identity_without_effects() {
        for error in [
            activation_status([0; 32], Principal::anonymous(), false).err(),
            refill_status([0; 32], Principal::anonymous(), false).err(),
        ] {
            assert_eq!(
                error.unwrap().public_error().code(),
                canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
            );
        }
    }

    #[test]
    fn synchronization_read_rejects_zero_identity_without_state_access() {
        let error =
            registry_synchronization_status([0; 32], Principal::anonymous(), false).unwrap_err();
        assert_eq!(
            error.public_error().code(),
            canic_contracts::diagnostics::codes::REQUEST_INVALID.raw_code()
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
            canic_contracts::diagnostics::codes::STATE_INVALID.raw_code()
        );
    }
}
