//! Module: fleet_ensure::ops::current_protocol::component_progress
//!
//! Responsibility: project reviewed names and exact sequential member cursors.
//! Boundary: no extra reads and no readiness inferred from aggregate Root counts.

#[cfg(test)]
pub(super) mod tests;

use crate::fleet_ensure::dto::{FleetComponentProgress, FleetComponentProgressState as State};
use candid::Principal;
use canic_contracts::dto::component_provisioning::{
    FleetComponentProvisioningPlan, FleetComponentProvisioningStatusResponse,
};

pub(super) fn project(
    plan: &FleetComponentProvisioningPlan,
    status: &FleetComponentProvisioningStatusResponse,
) -> Vec<FleetComponentProgress> {
    let mut components = Vec::new();
    for batch in &plan.batches {
        let root = batch.root.fleet_subnet_root;
        let count = batch
            .placements
            .iter()
            .map(|placement| placement.entries.len())
            .sum();
        let mut index = 0;
        for placement in &batch.placements {
            for entry in &placement.entries {
                let (state, current) = member_state(root, index, count, status);
                components.push(FleetComponentProgress {
                    component_spec: entry.component_spec.to_string(),
                    deployment: placement.group_placement.deployment.to_string(),
                    placement: placement.group_placement.ordinal,
                    member_path: entry
                        .member_path
                        .as_slice()
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    root,
                    state,
                    current,
                });
                index += 1;
            }
        }
    }
    components
}

fn member_state(
    root: Principal,
    index: usize,
    count: usize,
    status: &FleetComponentProvisioningStatusResponse,
) -> (State, bool) {
    if let Some(cursor) = status
        .current_activation
        .filter(|cursor| cursor.fleet_subnet_root == root)
    {
        if cursor.component_count as usize != count
            || cursor.activated_component_count > cursor.component_count
        {
            return (State::Unknown, false);
        }
        return if index < cursor.activated_component_count as usize {
            (State::Active, false)
        } else {
            (
                State::RuntimePending,
                index == cursor.activated_component_count as usize,
            )
        };
    }
    if let Some(cursor) = status
        .current_publication
        .filter(|cursor| cursor.fleet_subnet_root == root)
    {
        if cursor.component_count as usize != count
            || cursor.published_component_count > cursor.component_count
        {
            return (State::Unknown, false);
        }
        return if index < cursor.published_component_count as usize {
            (State::Published, false)
        } else {
            (
                State::Registered,
                index == cursor.published_component_count as usize,
            )
        };
    }
    let Some(cursor) = status
        .current_root
        .filter(|cursor| cursor.fleet_subnet_root == root)
    else {
        // Aggregate completed-Root counts do not identify a member observation.
        return (State::Unknown, false);
    };
    let counts = [
        cursor.registry_committed_component_count,
        cursor.installed_component_count,
        cursor.claimed_component_count,
        cursor.reserved_component_count,
        cursor.component_count,
    ];
    if cursor.component_count as usize != count || counts.windows(2).any(|pair| pair[0] > pair[1]) {
        return (State::Unknown, false);
    }
    let state = counts
        .iter()
        .zip([
            State::Registered,
            State::Installed,
            State::Claimed,
            State::Reserved,
        ])
        .find_map(|(completed, state)| (index < *completed as usize).then_some(state))
        .unwrap_or(State::Unknown);
    let current = counts[..4]
        .iter()
        .rev()
        .find(|completed| (**completed as usize) < count)
        .is_some_and(|completed| index == *completed as usize);
    (state, current)
}
