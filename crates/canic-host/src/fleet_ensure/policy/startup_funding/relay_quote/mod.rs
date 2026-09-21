//! Module: fleet_ensure::policy::startup_funding::relay_quote
//!
//! Responsibility: quote bounded balance and ledger observations for every qualified funding edge.
//! Boundary: proposed allowances are not approval, measured burn or durable retry authority.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::DesiredFleet,
    policy::{
        canister_cycle_policy, cycle_bounds,
        startup_funding::{live_binding, recovery_minimum_cycles},
    },
    view::startup_funding::{
        StartupDemandUnavailable, StartupNativeBalance, StartupRelayQuote, StartupRootFunding,
        StartupUsageUnavailable,
    },
};

#[cfg(test)]
pub(in crate::fleet_ensure) use tests::qualify;

/// Quote the selected current inventory without spending or depending on descendant ledgers.
pub(in crate::fleet_ensure) fn project(
    desired: &DesiredFleet,
    root: &StartupRootFunding,
) -> Result<StartupRelayQuote, StartupDemandUnavailable> {
    let coverage = root.inventory.map_err(StartupDemandUnavailable::Usage)?;
    let unavailable =
        || StartupDemandUnavailable::Usage(StartupUsageUnavailable::AuthorityMismatch);
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(unavailable)?;
    let configured = desired
        .canisters
        .iter()
        .find(|entry| entry.name == root.root)
        .ok_or_else(unavailable)?;
    let principal = configured
        .principal
        .as_ref()
        .and_then(|id| id.parse().ok())
        .ok_or_else(unavailable)?;
    let bindings = root
        .child_usage
        .iter()
        .filter(|child| child.usage != Err(StartupUsageUnavailable::NotWorkload))
        .map(|child| {
            child.binding.as_ref().ok_or_else(|| {
                StartupDemandUnavailable::Usage(
                    child
                        .usage
                        .as_ref()
                        .err()
                        .copied()
                        .unwrap_or(StartupUsageUnavailable::InventoryIncomplete),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let graph = live_binding::graph(
        &bootstrap
            .component_deployment_configuration
            .component_topology,
        bootstrap.release_build_id,
        bindings.iter().copied(),
    );
    for result in graph.values() {
        (*result).map_err(StartupDemandUnavailable::Usage)?;
    }
    if bindings
        .iter()
        .any(|binding| binding.component.fleet_subnet_root != principal)
    {
        return Err(unavailable());
    }
    for binding in &bindings {
        live_binding::selected(desired, &root.root, binding)
            .map_err(StartupDemandUnavailable::Usage)?;
    }
    let descendants = bindings
        .iter()
        .filter(|binding| binding.parent != principal)
        .count();
    let mut requests = bindings
        .iter()
        .map(|binding| (*binding).clone())
        .collect::<Vec<_>>();
    if descendants != coverage.descendants || bindings.len() - descendants != coverage.components {
        return Err(StartupDemandUnavailable::Usage(
            StartupUsageUnavailable::InventoryIncomplete,
        ));
    }
    requests.sort_by_key(|binding| (binding.parent, binding.canister_id));
    let bounds = observation_bounds(desired, &root.root, !requests.is_empty())?;
    calculate(
        requests,
        root.balance,
        bounds.recovery_floor_cycles,
        bounds.per_attempt_cycles,
    )
}

/// Use the same configured margins and minimum recovery policy when restoring an approval.
pub(in crate::fleet_ensure) fn observation_bounds(
    desired: &DesiredFleet,
    root: &str,
    paid: bool,
) -> Result<
    crate::fleet_ensure::view::startup_funding::StartupObservationBounds,
    StartupDemandUnavailable,
> {
    let unavailable =
        || StartupDemandUnavailable::Usage(StartupUsageUnavailable::AuthorityMismatch);
    let configured = desired
        .canisters
        .iter()
        .find(|entry| entry.name == root)
        .ok_or_else(unavailable)?;
    let bounds =
        cycle_bounds(desired).map_err(|_| StartupDemandUnavailable::InvalidObservationBounds)?;
    let per_attempt_cycles = bounds
        .observation_burn
        .checked_add(bounds.update_burn)
        // One ledger relay and one management inspection, consumed as one non-retryable attempt.
        .and_then(|value| value.checked_mul(2))
        .ok_or(StartupDemandUnavailable::ArithmeticOverflow)?;
    if paid && (bounds.observation_burn == 0 || bounds.update_burn == 0) {
        return Err(StartupDemandUnavailable::InvalidObservationBounds);
    }
    let configured_minimum = canister_cycle_policy(configured)
        .map_err(|_| unavailable())?
        .minimum_cycles;
    let recovery_floor_cycles = recovery_minimum_cycles(desired, root, configured_minimum)
        .map_err(|_| StartupDemandUnavailable::ArithmeticOverflow)?;
    Ok(
        crate::fleet_ensure::view::startup_funding::StartupObservationBounds {
            per_attempt_cycles,
            recovery_floor_cycles,
        },
    )
}

fn calculate(
    requests: Vec<crate::fleet_ensure::view::startup_funding::StartupChildFundingBinding>,
    balance: StartupNativeBalance,
    minimum: u128,
    per_attempt: u128,
) -> Result<StartupRelayQuote, StartupDemandUnavailable> {
    let StartupNativeBalance::Observed(balance) = balance else {
        return Err(StartupDemandUnavailable::BalanceNotObserved);
    };
    let proposed_burn_allowance_cycles = per_attempt
        .checked_mul(requests.len() as u128)
        .ok_or(StartupDemandUnavailable::ArithmeticOverflow)?;
    let required_native_cycles = minimum
        .checked_add(proposed_burn_allowance_cycles)
        .ok_or(StartupDemandUnavailable::ArithmeticOverflow)?;
    Ok(StartupRelayQuote {
        requests,
        proposed_burn_allowance_cycles,
        per_attempt_burn_allowance_cycles: per_attempt,
        minimum_recovery_cycles: minimum,
        required_native_cycles,
        observed_native_cycles: balance,
        native_shortfall_cycles: required_native_cycles.saturating_sub(balance),
    })
}
