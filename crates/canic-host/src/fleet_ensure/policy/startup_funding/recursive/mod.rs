//! Module: fleet_ensure::policy::startup_funding::recursive
//!
//! Responsibility: calculate live parent demand from a complete qualified funding tree.
//! Boundary: missing balances, unsettled grants and unavailable accounting never become zero.

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(in crate::fleet_ensure) use tests::qualify;

use crate::fleet_ensure::{
    model::DesiredFleet,
    policy::startup_funding::{allowance, grant_demand, relay_quote},
    view::startup_funding::{
        StartupDemandUnavailable, StartupNativeBalance, StartupRecoveryDemand,
        StartupRecursiveChildDemand, StartupRootFunding, StartupUsageUnavailable,
    },
};
use canic_core::bootstrap::compiled::ConfigModel;
use std::collections::BTreeMap;

/// Forecast a settled grant wave, preserving the Root recovery floor above outgoing transfers.
pub(in crate::fleet_ensure) fn project(
    config: &ConfigModel,
    desired: &DesiredFleet,
    root: &StartupRootFunding,
) -> Result<StartupRecoveryDemand, StartupDemandUnavailable> {
    // This independently checks complete membership, selected placement and every parent edge.
    let quote = relay_quote::project(desired, root)?;
    let invalid = StartupDemandUnavailable::Usage(StartupUsageUnavailable::AuthorityMismatch);
    let bootstrap = desired.bootstrap.as_ref().ok_or(invalid)?;
    let topology = &bootstrap
        .component_deployment_configuration
        .component_topology;
    let entries = root
        .child_usage
        .iter()
        .filter(|entry| entry.usage != Err(StartupUsageUnavailable::NotWorkload))
        .collect::<Vec<_>>();
    let mut results = BTreeMap::new();
    while results.len() < entries.len() {
        let before = results.len();
        for entry in &entries {
            let binding = entry.binding.as_ref().ok_or(invalid)?;
            if results.contains_key(&binding.canister_id) {
                continue;
            }
            let children = entries.iter().filter(|child| {
                child
                    .binding
                    .as_ref()
                    .is_some_and(|child| child.parent == binding.canister_id)
            });
            let mut outgoing = 0u128;
            let mut ready = true;
            for child in children {
                let id = child.binding.as_ref().ok_or(invalid)?.canister_id;
                let Some(child): Option<&StartupRecursiveChildDemand> = results.get(&id) else {
                    ready = false;
                    break;
                };
                outgoing = checked_add(outgoing, child.parent_grants_cycles)?;
            }
            if !ready {
                continue;
            }
            let result = edge(
                config,
                topology,
                bootstrap.release_build_id,
                entry,
                outgoing,
            )?;
            results.insert(binding.canister_id, result);
        }
        if before == results.len() {
            return Err(invalid);
        }
    }
    let root_principal = desired
        .canisters
        .iter()
        .find(|canister| canister.name == root.root)
        .and_then(|canister| canister.principal.as_deref())
        .and_then(|id| id.parse().ok())
        .ok_or(invalid)?;
    let root_grants_cycles = results
        .values()
        .filter(|entry| entry.parent == root_principal)
        .try_fold(0, |total, entry| {
            checked_add(total, entry.parent_grants_cycles)
        })?;
    let uncovered_cycles = results
        .values()
        .try_fold(0, |total, entry| checked_add(total, entry.uncovered_cycles))?;
    let minimum_native_cycles = checked_add(quote.minimum_recovery_cycles, root_grants_cycles)?;
    let StartupNativeBalance::Observed(balance) = root.balance else {
        return Err(StartupDemandUnavailable::BalanceNotObserved);
    };
    Ok(StartupRecoveryDemand {
        exceeds_root_window_budget: root_grants_cycles
            > root.funding_budget.maximum_cycles.to_u128(),
        root_grants_cycles,
        minimum_native_cycles,
        shortfall_cycles: minimum_native_cycles.saturating_sub(balance),
        uncovered_cycles,
        children: results.into_values().collect(),
    })
}

fn checked_add(left: u128, right: u128) -> Result<u128, StartupDemandUnavailable> {
    left.checked_add(right)
        .ok_or(StartupDemandUnavailable::ArithmeticOverflow)
}

fn edge(
    config: &ConfigModel,
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    release: canic_core::ids::ReleaseBuildId,
    entry: &crate::fleet_ensure::view::startup_funding::StartupChildFundingUsage,
    outgoing: u128,
) -> Result<StartupRecursiveChildDemand, StartupDemandUnavailable> {
    let invalid = StartupDemandUnavailable::Usage(StartupUsageUnavailable::AuthorityMismatch);
    let binding = entry.binding.as_ref().ok_or(invalid)?;
    let usage = entry
        .usage
        .as_ref()
        .map_err(|reason| StartupDemandUnavailable::Usage(*reason))?;
    let allowance = allowance::project(config, topology, release, binding, usage)
        .map_err(StartupDemandUnavailable::Usage)?;
    if usage.pending_operations > 0 {
        return Err(StartupDemandUnavailable::PendingGrant);
    }
    let balance = entry
        .observed_balance_cycles
        .ok_or(StartupDemandUnavailable::BalanceNotObserved)?;
    let policy = config
        .component_specs
        .get(&binding.component.component_spec)
        .and_then(|spec| spec.get_canister(&binding.role))
        .ok_or(invalid)?;
    let threshold = policy.topup.as_ref().map(|topup| topup.threshold.to_u128());
    let grant = policy.topup.as_ref().map_or(0, |topup| {
        topup
            .amount
            .to_u128()
            .min(policy.cycles_funding.max_per_request.to_u128())
    });
    let demand = grant_demand(
        balance,
        outgoing,
        threshold,
        grant,
        allowance.remaining_after_charges_cycles,
    )
    .map_err(|_| StartupDemandUnavailable::ArithmeticOverflow)?;
    Ok(StartupRecursiveChildDemand {
        exceeds_component_window_budget: binding.canister_id == binding.component.canister_id
            && outgoing
                > config.component_specs[&binding.component.component_spec]
                    .limits
                    .cycles_funding
                    .maximum_cycles
                    .to_u128(),
        child: binding.canister_id,
        parent: binding.parent,
        outgoing_cycles: outgoing,
        parent_grants_cycles: demand.cycles,
        uncovered_cycles: demand.unfunded,
        cooldown_remaining_secs: allowance.cooldown_remaining_secs,
    })
}
