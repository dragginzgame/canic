//! Module: fleet_ensure::ops::startup_funding::observation::binding
//!
//! Responsibility: resolve an asset's funding parent from its current Root allocation claim.
//! Boundary: a physical pool owner is not necessarily the logical funding parent.

#[cfg(test)]
mod tests;

use crate::{
    canister_protocol::query_with_candid,
    fleet_ensure::view::startup_funding::{StartupChildFundingBinding, StartupUsageUnavailable},
    icp::IcpCli,
};
use candid::Principal;
use canic_contracts::dto::component_registry::RootComponentAllocationPhase;
use canic_contracts::dto::fleet_coordinator::CoordinatorOperationReadRequest as Request;
use canic_contracts::dto::pool::CanisterPoolAssetStatus;
use canic_contracts::dto::pool::CanisterPoolClaim;
use canic_contracts::dto::role::OperationStatusRequest;
use canic_contracts::dto::root::RootOperationStatusResponse;
use canic_contracts::dto::wire::projection::startup_binding::Response;
use canic_contracts::ids::CanisterRole;
use canic_contracts::ids::ComponentBinding;
use canic_contracts::ids::ComponentInstanceId;
use canic_contracts::ids::ComponentSpecId;
use canic_contracts::ids::FleetSubnetRootReleaseSet;
use canic_contracts::protocol;
use std::path::Path;

/// Query the exact allocation behind a current Workload; never guess a parent for other assets.
pub fn observe(
    icp: &IcpCli,
    candid: &Path,
    root: Principal,
    child: Principal,
    status: &CanisterPoolAssetStatus,
) -> Result<StartupChildFundingBinding, StartupUsageUnavailable> {
    let CanisterPoolAssetStatus::Workload { claim } = status else {
        return Err(StartupUsageUnavailable::NotWorkload);
    };
    let Response::Operation(response) = query_with_candid(
        icp,
        candid,
        root,
        protocol::CANIC_ROOT_OPERATION_STATUS,
        &Request::Operation(OperationStatusRequest {
            operation_id: claim.operation_id,
        }),
    )
    .map_err(|_| StartupUsageUnavailable::ObservationFailed)?;
    project(root, child, claim, *response)
}

#[derive(Eq, PartialEq)]
struct AllocationIdentity {
    operation: [u8; 32],
    component: ComponentInstanceId,
    root: Principal,
    child: Principal,
}

#[derive(Eq, PartialEq)]
struct ComponentIdentity {
    component: ComponentInstanceId,
    spec: ComponentSpecId,
    spec_hash: [u8; 32],
    role: CanisterRole,
}

#[derive(Eq, PartialEq)]
struct ChildIdentity {
    component: ComponentInstanceId,
    parent: Principal,
    role: CanisterRole,
}

fn project(
    root: Principal,
    child: Principal,
    claim: &CanisterPoolClaim,
    response: RootOperationStatusResponse,
) -> Result<StartupChildFundingBinding, StartupUsageUnavailable> {
    let invalid = StartupUsageUnavailable::AuthorityMismatch;
    let (actual, component, parent, parent_role, role, release_set) = match response {
        RootOperationStatusResponse::ProvisionComponent(status) => {
            let allocation = status.allocation;
            let install = allocation.installation.ok_or(invalid)?;
            if !status.complete || allocation.phase != RootComponentAllocationPhase::Committed {
                return Err(StartupUsageUnavailable::PolicyTransition);
            }
            let reserved = ComponentIdentity {
                component: allocation.component,
                spec: allocation.component_spec,
                spec_hash: allocation.spec_hash,
                role: allocation.role,
            };
            let installed = ComponentIdentity {
                component: install.binding.component,
                spec: install.binding.component_spec.clone(),
                spec_hash: install.binding.spec_hash,
                role: install.binding.role.clone(),
            };
            if reserved != installed {
                return Err(invalid);
            }
            let actual = AllocationIdentity {
                operation: allocation.operation_id,
                component: allocation.component,
                root: install.binding.fleet_subnet_root,
                child: install.binding.canister_id,
            };
            let role = install.binding.role.clone();
            (
                actual,
                install.binding,
                root,
                None,
                role,
                allocation.release_set,
            )
        }
        RootOperationStatusResponse::ProvisionChild(status) => {
            let allocation = status.allocation;
            let install = allocation.installation.ok_or(invalid)?;
            if allocation.phase != RootComponentAllocationPhase::Committed {
                return Err(StartupUsageUnavailable::PolicyTransition);
            }
            let reserved = ChildIdentity {
                component: allocation.component,
                parent: allocation.parent_canister_id,
                role: allocation.child_role,
            };
            let installed = ChildIdentity {
                component: install.binding.component.component,
                parent: install.binding.parent_canister_id,
                role: install.binding.role.clone(),
            };
            if reserved != installed {
                return Err(invalid);
            }
            let actual = AllocationIdentity {
                operation: allocation.operation_id,
                component: allocation.component,
                root: install.binding.component.fleet_subnet_root,
                child: install.binding.canister_id,
            };
            (
                actual,
                install.binding.component,
                install.binding.parent_canister_id,
                Some(allocation.parent_role),
                install.binding.role,
                allocation.release_set,
            )
        }
        _ => return Err(invalid),
    };
    if actual
        != (AllocationIdentity {
            operation: claim.operation_id,
            component: claim.component,
            root,
            child,
        })
    {
        return Err(invalid);
    }
    finish(component, parent, parent_role, child, role, release_set)
}

fn finish(
    component: ComponentBinding,
    parent: Principal,
    parent_role: Option<CanisterRole>,
    child: Principal,
    role: CanisterRole,
    release_set: FleetSubnetRootReleaseSet,
) -> Result<StartupChildFundingBinding, StartupUsageUnavailable> {
    if parent == child
        || parent == Principal::anonymous()
        || parent == Principal::management_canister()
    {
        return Err(StartupUsageUnavailable::AuthorityMismatch);
    }
    Ok(StartupChildFundingBinding {
        release_set,
        component,
        canister_id: child,
        parent,
        parent_role,
        role,
    })
}
