//! Module: fleet_ensure::ops::startup_funding::observation::binding
//!
//! Responsibility: resolve an asset's funding parent from its current Root allocation claim.
//! Boundary: a physical pool owner is not necessarily the logical funding parent.

#[cfg(test)]
mod tests;

use crate::{
    canister_protocol::{CanisterProtocolError, query_wire_cli},
    fleet_ensure::view::startup_funding::{StartupChildFundingBinding, StartupUsageUnavailable},
    icp::IcpCli,
};
use canic_contracts::{
    dto::{
        component_read,
        component_registry::RootComponentAllocationPhase,
        component_registry::RootComponentChildAllocationResponse,
        pool::{CanisterPoolAssetStatus, CanisterPoolClaim},
        root::RootComponentOperationStatus,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentInstanceId, ComponentSpecId,
        FleetSubnetRootReleaseSet,
    },
    protocol,
};
use ic_platform::principal::ExternalPrincipal as Principal;

/// Query the exact allocation behind a current Workload; never guess a parent for other assets.
pub fn observe(
    icp: &IcpCli,
    root: Principal,
    child: Principal,
    status: &CanisterPoolAssetStatus,
) -> Result<StartupChildFundingBinding, StartupUsageUnavailable> {
    let CanisterPoolAssetStatus::Workload { claim } = status else {
        return Err(StartupUsageUnavailable::NotWorkload);
    };
    let top: Result<component_read::ComponentAllocation, CanisterProtocolError> = query_wire_cli(
        icp,
        root,
        protocol::component_read::allocation::METHOD,
        protocol::component_read::allocation::METHOD_ID,
        &claim.operation_id,
        protocol::component_read::REQUEST_LIMITS,
        protocol::component_read::RESPONSE_LIMITS,
    );
    let top = optional(top)?;
    let descendant: Result<component_read::ChildAllocation, CanisterProtocolError> = query_wire_cli(
        icp,
        root,
        protocol::component_read::child_allocation::METHOD,
        protocol::component_read::child_allocation::METHOD_ID,
        &claim.operation_id,
        protocol::component_read::REQUEST_LIMITS,
        protocol::component_read::RESPONSE_LIMITS,
    );
    let descendant = optional(descendant)?;
    let evidence = match (top, descendant) {
        (Some(top), None) => AllocationEvidence::Component(top.into()),
        (None, Some(child)) => AllocationEvidence::Child(child.into()),
        _ => return Err(StartupUsageUnavailable::AuthorityMismatch),
    };
    project(root, child, claim, evidence)
}

/// Exactly one domain must own the observed allocation operation.
enum AllocationEvidence {
    Component(RootComponentOperationStatus),
    Child(RootComponentChildAllocationResponse),
}

fn optional<T>(
    response: Result<T, CanisterProtocolError>,
) -> Result<Option<T>, StartupUsageUnavailable> {
    match response {
        Ok(value) => Ok(Some(value)),
        Err(error)
            if error.is_rejected_with(canic_contracts::diagnostics::codes::STATE_UNAVAILABLE) =>
        {
            Ok(None)
        }
        Err(_) => Err(StartupUsageUnavailable::ObservationFailed),
    }
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
    response: AllocationEvidence,
) -> Result<StartupChildFundingBinding, StartupUsageUnavailable> {
    let invalid = StartupUsageUnavailable::AuthorityMismatch;
    let (actual, component, parent, parent_role, role, release_set) = match response {
        AllocationEvidence::Component(status) => {
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
        AllocationEvidence::Child(allocation) => {
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
