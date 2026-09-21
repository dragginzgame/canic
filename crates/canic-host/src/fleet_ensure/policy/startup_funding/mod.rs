//! Module: fleet_ensure::policy::startup_funding
//!
//! Responsibility: forecast initial grant demand over the compiled parent/child graph.
//! Does not own: live observations, grant history, mutation or spending authority.
//! Boundary: assumes pool-readiness balances, fresh grant ledgers and no execution burn.

pub(in crate::fleet_ensure) mod allowance;
pub(in crate::fleet_ensure) mod live_binding;
pub(in crate::fleet_ensure) mod local_demand;
pub(super) mod planning;
pub(in crate::fleet_ensure) mod recursive;
pub(in crate::fleet_ensure) mod relay_quote;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(in crate::fleet_ensure) use planning::tests::{qualify, qualify_creation};
#[cfg(test)]
pub(in crate::fleet_ensure) use tests::{funding_binding, hub_config, hub_source};

use super::{EnsurePolicyError, initial_role_instances};
use crate::fleet_ensure::{
    model::{DesiredFleet, DesiredFleetBootstrapRoot},
    view::startup_funding::{StartupComponentFunding, StartupRoleFunding},
};
use canic_core::{
    bootstrap::compiled::ConfigModel, control_plane_support::config::ComponentSpec,
    ids::CanisterRole,
};
use std::collections::BTreeMap;

/// Retain both automatic funding headroom and the deployment guard after child grants.
pub(in crate::fleet_ensure) fn minimum_root_cycles(
    request_threshold: u128,
    child_grants: u128,
) -> Result<u128, EnsurePolicyError> {
    use canic_core::control_plane_support::policy::deployment::MINIMUM_DEPLOYMENT_RESERVE_CYCLES;
    add(
        add(request_threshold, 1)?.max(MINIMUM_DEPLOYMENT_RESERVE_CYCLES),
        child_grants,
    )
}

/// Recover above the selected Root's request threshold without lowering its desired minimum.
pub(in crate::fleet_ensure) fn recovery_minimum_cycles(
    desired: &DesiredFleet,
    root: &str,
    configured_minimum: u128,
) -> Result<u128, EnsurePolicyError> {
    let threshold = if let Some(bootstrap) = &desired.bootstrap {
        let mut matches = bootstrap.roots.iter().filter(|entry| entry.root == root);
        let selected = matches.next().ok_or_else(|| {
            invalid_topology("native recovery requires the selected Root's funding authority")
        })?;
        if matches.next().is_some() {
            return Err(invalid_topology(
                "native recovery requires one Root funding authority",
            ));
        }
        selected.funding.root_funding.request_threshold.to_u128()
    } else {
        0
    };
    Ok(configured_minimum.max(minimum_root_cycles(threshold, 0)?))
}

/// Compile exactly the configured initial placements belonging to one Root.
pub(in crate::fleet_ensure) fn root_components(
    config: &ConfigModel,
    desired: &DesiredFleet,
    root: &DesiredFleetBootstrapRoot,
) -> Result<Vec<StartupComponentFunding>, EnsurePolicyError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or_else(|| invalid_topology("startup funding requires bootstrap"))?;
    let protocol = desired
        .protocol
        .as_ref()
        .ok_or_else(|| invalid_topology("startup funding requires protocol"))?;
    let mut components = Vec::new();
    for placement in protocol
        .component_group_placements
        .iter()
        .filter(|placement| placement.root == root.root)
    {
        let deployment = bootstrap
            .component_deployment_configuration
            .deployment_topology
            .component_group_deployments
            .iter()
            .find(|deployment| deployment.deployment.as_str() == placement.deployment)
            .ok_or_else(|| invalid_topology("startup funding requires a configured deployment"))?;
        for member in &deployment.members {
            let spec = bootstrap
                .component_deployment_configuration
                .component_topology
                .component_specs
                .iter()
                .find(|spec| spec.component_spec == member.component_spec)
                .ok_or_else(|| {
                    invalid_topology("startup funding requires a configured Component Spec")
                })?;
            components.push(component(
                config,
                spec,
                root.limits.canister_pool.canister_cycles.to_u128(),
                &placement.deployment,
                placement.ordinal,
            )?);
        }
    }
    Ok(components)
}

/// Compile one Component instance from leaves to parent, counting each transfer once per edge.
pub(in crate::fleet_ensure) fn component(
    config: &ConfigModel,
    spec: &ComponentSpec,
    pool_balance: u128,
    deployment: &str,
    ordinal: u32,
) -> Result<StartupComponentFunding, EnsurePolicyError> {
    let roles = compile_roles(config, spec, pool_balance)?;
    let root_grant_cycles = roles[&spec.component_role].parent_grants_per_instance_cycles;
    let descendant_grants_cycles = roles
        .values()
        .filter(|role| role.role != spec.component_role)
        .try_fold(0, |total, role| {
            add(
                total,
                multiply(
                    role.parent_grants_per_instance_cycles,
                    u128::from(role.instances),
                )?,
            )
        })?;
    Ok(StartupComponentFunding {
        component_spec: spec.component_spec.clone(),
        deployment: deployment.to_owned(),
        descendant_grants_cycles,
        exceeds_window_budget: descendant_grants_cycles
            > spec.limits.cycles_funding.maximum_cycles.to_u128(),
        funding_budget: spec.limits.cycles_funding.clone(),
        ordinal,
        roles: roles.into_values().collect(),
        root_grant_cycles,
    })
}

fn compile_roles(
    config: &ConfigModel,
    spec: &ComponentSpec,
    pool_balance: u128,
) -> Result<BTreeMap<CanisterRole, StartupRoleFunding>, EnsurePolicyError> {
    let source = config
        .component_specs
        .get(&spec.component_spec)
        .ok_or_else(|| {
            invalid_topology("startup funding has no matching configured Component Spec")
        })?;
    let mut instances = initial_role_instances(spec)?;
    instances.retain(|_, count| *count > 0);
    let mut roles = BTreeMap::<CanisterRole, StartupRoleFunding>::new();
    while roles.len() < instances.len() {
        let before = roles.len();
        for (role, count) in &instances {
            if roles.contains_key(role) {
                continue;
            }
            let Some(outgoing) = outgoing_grants(spec, role, &roles)? else {
                continue;
            };
            let policy = source.get_canister(role).ok_or_else(|| {
                invalid_topology("startup funding has no matching configured role")
            })?;
            let threshold = policy.topup.as_ref().map(|topup| topup.threshold.to_u128());
            let grant = policy.topup.as_ref().map_or(0, |topup| {
                topup
                    .amount
                    .to_u128()
                    .min(policy.cycles_funding.max_per_request.to_u128())
            });
            let demand = grant_demand(
                pool_balance,
                outgoing,
                threshold,
                grant,
                policy.cycles_funding.max_per_child.to_u128(),
            )?;
            roles.insert(
                role.clone(),
                StartupRoleFunding {
                    cooldown_secs: policy.cycles_funding.cooldown_secs,
                    effective_grant_cycles: grant,
                    grants_per_instance: demand.count,
                    initial_balance_cycles: pool_balance,
                    instances: *count,
                    maximum_per_child_cycles: policy.cycles_funding.max_per_child.to_u128(),
                    outgoing_grants_per_instance_cycles: outgoing,
                    parent_grants_per_instance_cycles: demand.cycles,
                    role: role.clone(),
                    threshold_cycles: threshold,
                    unfunded_per_instance_cycles: demand.unfunded,
                },
            );
        }
        if roles.len() == before {
            return Err(invalid_topology(
                "startup funding role graph did not converge",
            ));
        }
    }
    Ok(roles)
}

fn outgoing_grants(
    spec: &ComponentSpec,
    role: &CanisterRole,
    roles: &BTreeMap<CanisterRole, StartupRoleFunding>,
) -> Result<Option<u128>, EnsurePolicyError> {
    let mut outgoing = 0_u128;
    for edge in spec
        .spawn_grants
        .iter()
        .filter(|edge| &edge.parent_role == role && edge.initial_instances_per_parent > 0)
    {
        let Some(child) = roles.get(&edge.child_role) else {
            return Ok(None);
        };
        outgoing = add(
            outgoing,
            multiply(
                child.parent_grants_per_instance_cycles,
                u128::from(edge.initial_instances_per_parent),
            )?,
        )?;
    }
    Ok(Some(outgoing))
}

/// Rounded funding admitted by a fresh per-child ledger, excluding time and burn.
#[derive(Debug, Eq, PartialEq)]
struct GrantDemand {
    count: u128,
    cycles: u128,
    unfunded: u128,
}

fn grant_demand(
    balance: u128,
    outgoing: u128,
    threshold: Option<u128>,
    grant: u128,
    lifetime_limit: u128,
) -> Result<GrantDemand, EnsurePolicyError> {
    // Runtime requests at equality too: reaching the threshold does not finish a wave.
    let target = threshold.map_or(Ok(0), |threshold| add(threshold, 1))?;
    let required = add(outgoing, target)?.saturating_sub(balance);
    if threshold.is_none() || grant == 0 || required == 0 {
        return Ok(GrantDemand {
            count: 0,
            cycles: 0,
            unfunded: required,
        });
    }
    let requested = multiply(required.div_ceil(grant), grant)?;
    let cycles = requested.min(lifetime_limit);
    Ok(GrantDemand {
        count: cycles.div_ceil(grant),
        cycles,
        unfunded: required.saturating_sub(cycles),
    })
}

pub(in crate::fleet_ensure) fn add(left: u128, right: u128) -> Result<u128, EnsurePolicyError> {
    left.checked_add(right)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "startup funding demand",
        })
}

fn multiply(left: u128, right: u128) -> Result<u128, EnsurePolicyError> {
    left.checked_mul(right)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "startup funding demand",
        })
}

fn invalid_topology(reason: &str) -> EnsurePolicyError {
    EnsurePolicyError::EstateFundingTopology {
        reason: reason.to_owned(),
    }
}
