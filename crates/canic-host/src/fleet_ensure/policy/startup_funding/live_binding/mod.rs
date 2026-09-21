//! Module: fleet_ensure::policy::startup_funding::live_binding
//!
//! Responsibility: qualify observed funding edges against selected placement and Component identity.
//! Does not own: observation, grant accounting, complete inventory discovery or spending authority.
//! Boundary: an absent parent is unknown; pool custody never substitutes for a funding edge.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::DesiredFleet,
    view::startup_funding::{
        StartupChildFundingBinding, StartupFundingPlacement, StartupFundingRegistry,
        StartupUsageUnavailable,
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::config::ComponentTopology,
    ids::{
        CanisterRole, FleetBinding, FleetCoordinatorBinding, FleetKey, ReleaseBuildId, SubnetId,
    },
};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
pub(in crate::fleet_ensure) use tests::qualify_selected;

#[derive(Eq, PartialEq)]
struct PlacementAuthority {
    coordinator: FleetCoordinatorBinding,
    root: Principal,
    subnet: SubnetId,
}

#[derive(Eq, PartialEq)]
struct FundingEdge<'a> {
    parent: Principal,
    child: Principal,
    role: &'a CanisterRole,
}

/// Bind an allocation to independently observed current authority and selected Root policy.
pub(in crate::fleet_ensure) fn current(
    desired: &DesiredFleet,
    root_name: &str,
    binding: &StartupChildFundingBinding,
    registry: &StartupFundingRegistry,
) -> Result<(), StartupUsageUnavailable> {
    selected(desired, root_name, binding)?;
    let invalid = StartupUsageUnavailable::AuthorityMismatch;
    if binding.component.authority != registry.authority {
        return Err(invalid);
    }
    let placement = registry
        .roots
        .get(&binding.component.fleet_subnet_root)
        .ok_or(invalid)?;
    let root = desired
        .bootstrap
        .as_ref()
        .ok_or(invalid)?
        .roots
        .iter()
        .find(|root| root.root == root_name)
        .ok_or(invalid)?;
    let expected = StartupFundingPlacement {
        active: true,
        placement_subnet: root.placement_subnet,
        release_set: binding.release_set,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
        funding: root.funding.clone(),
    };
    if *placement != expected {
        return Err(StartupUsageUnavailable::PolicyTransition);
    }
    Ok(())
}

/// Match the selected Fleet and Root placement before interpreting an allocation response.
pub(in crate::fleet_ensure) fn selected(
    desired: &DesiredFleet,
    root_name: &str,
    binding: &StartupChildFundingBinding,
) -> Result<(), StartupUsageUnavailable> {
    let invalid = StartupUsageUnavailable::AuthorityMismatch;
    let bootstrap = desired.bootstrap.as_ref().ok_or(invalid)?;
    let root = bootstrap
        .roots
        .iter()
        .find(|root| root.root == root_name)
        .ok_or(invalid)?;
    if !root.component_admissions.iter().any(|admission| {
        admission.component_spec == binding.component.component_spec
            && admission.maximum_root_instances > 0
    }) {
        return Err(invalid);
    }
    let principal = |name: &str| {
        desired
            .canisters
            .iter()
            .find(|canister| canister.name == name)
            .and_then(|canister| canister.principal.as_deref())
            .and_then(|principal| principal.parse().ok())
            .ok_or(invalid)
    };
    let expected = PlacementAuthority {
        coordinator: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: bootstrap.canonical_network_id,
                    fleet_id: bootstrap.fleet_id,
                },
                app: bootstrap.app.clone(),
            },
            coordinator_subnet: bootstrap.coordinator_subnet,
            coordinator: principal(&bootstrap.coordinator)?,
        },
        root: principal(root_name)?,
        subnet: root.placement_subnet,
    };
    let actual = PlacementAuthority {
        coordinator: binding.component.authority.binding.clone(),
        root: binding.component.fleet_subnet_root,
        subnet: binding.component.placement_subnet,
    };
    if expected != actual || binding.component.authority.epoch == 0 {
        return Err(invalid);
    }
    declared(
        &bootstrap
            .component_deployment_configuration
            .component_topology,
        bootstrap.release_build_id,
        binding,
    )
}

/// Require the selected role contract and an explicit permitted funding edge.
pub(in crate::fleet_ensure) fn declared(
    topology: &ComponentTopology,
    release: ReleaseBuildId,
    binding: &StartupChildFundingBinding,
) -> Result<(), StartupUsageUnavailable> {
    let invalid = StartupUsageUnavailable::AuthorityMismatch;
    if binding.release_set.release_build_id != release {
        return Err(StartupUsageUnavailable::SelectedBuildNotInstalled);
    }
    let component = &binding.component;
    let spec = topology.get(&component.component_spec).ok_or(invalid)?;
    if spec.spec_hash != component.spec_hash {
        return Err(StartupUsageUnavailable::PolicyTransition);
    }
    if component.role != spec.component_role {
        return Err(invalid);
    }
    if binding.parent == binding.canister_id {
        return Err(invalid);
    }
    for principal in [
        binding.canister_id,
        binding.parent,
        component.canister_id,
        component.fleet_subnet_root,
    ] {
        if principal == Principal::anonymous() || principal == Principal::management_canister() {
            return Err(invalid);
        }
    }
    if [binding.canister_id, component.canister_id].contains(&component.fleet_subnet_root) {
        return Err(invalid);
    }
    match &binding.parent_role {
        None => {
            let actual = FundingEdge {
                parent: binding.parent,
                child: binding.canister_id,
                role: &binding.role,
            };
            let expected = FundingEdge {
                parent: component.fleet_subnet_root,
                child: component.canister_id,
                role: &component.role,
            };
            if actual != expected {
                return Err(invalid);
            }
        }
        Some(parent_role) => {
            let parent_is_root = binding.parent == component.fleet_subnet_root;
            let target_is_component = binding.canister_id == component.canister_id;
            if parent_is_root || target_is_component {
                return Err(invalid);
            }
            if !spec.spawn_grants.iter().any(|grant| {
                grant.parent_role == *parent_role
                    && grant.child_role == binding.role
                    && grant.maximum_instances_per_parent > 0
            }) {
                return Err(invalid);
            }
        }
    }
    Ok(())
}

/// Resolve each observed chain once; cycles, duplicates and cross-Component joins fail closed.
/// This validates observed edges, not completeness of the live estate.
pub(in crate::fleet_ensure) fn graph<'a>(
    topology: &ComponentTopology,
    release: ReleaseBuildId,
    bindings: impl Iterator<Item = &'a StartupChildFundingBinding>,
) -> BTreeMap<Principal, Result<(), StartupUsageUnavailable>> {
    let mut index = BTreeMap::new();
    for binding in bindings {
        index
            .entry(binding.canister_id)
            .and_modify(|value| *value = None)
            .or_insert(Some(binding));
    }
    let mut results = BTreeMap::new();
    for start in index.keys() {
        let mut cursor = *start;
        let mut path = Vec::new();
        let mut visited = BTreeSet::new();
        let result = loop {
            if let Some(result) = results.get(&cursor) {
                break *result;
            }
            if !visited.insert(cursor) {
                break Err(StartupUsageUnavailable::AuthorityMismatch);
            }
            path.push(cursor);
            let Some(Some(binding)) = index.get(&cursor) else {
                break Err(StartupUsageUnavailable::AuthorityMismatch);
            };
            if let Err(reason) = declared(topology, release, binding) {
                break Err(reason);
            }
            let Some(parent_role) = &binding.parent_role else {
                break Ok(());
            };
            let Some(parent) = index.get(&binding.parent) else {
                break Err(StartupUsageUnavailable::ParentNotObserved);
            };
            let Some(parent) = parent else {
                break Err(StartupUsageUnavailable::AuthorityMismatch);
            };
            if binding.component != parent.component
                || binding.release_set != parent.release_set
                || *parent_role != parent.role
            {
                break Err(StartupUsageUnavailable::AuthorityMismatch);
            }
            cursor = binding.parent;
        };
        for principal in path {
            results.insert(principal, result);
        }
    }
    results
}
