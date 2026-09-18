//! Module: fleet_ensure::policy::startup_funding::local_demand
//!
//! Responsibility: project one observed child's own threshold deficit against live charges.
//! Does not own: descendant demand, parent liquidity, observations or spending authority.
//! Boundary: pending transfers make demand unknown; snapshots do not predict execution burn.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    policy::startup_funding::allowance,
    view::startup_funding::{
        StartupChildAccounting, StartupChildFundingBinding, StartupChildLocalDemand,
        StartupDemandUnavailable, StartupUsageUnavailable,
    },
};
use canic_core::{
    bootstrap::compiled::ConfigModel, control_plane_support::config::ComponentTopology,
    ids::ReleaseBuildId,
};

/// Bind policy and charges before using native balance as evidence of local demand.
pub(in crate::fleet_ensure) fn project(
    config: &ConfigModel,
    topology: &ComponentTopology,
    release: ReleaseBuildId,
    binding: &StartupChildFundingBinding,
    usage: &StartupChildAccounting,
    balance: Option<u128>,
) -> Result<StartupChildLocalDemand, StartupDemandUnavailable> {
    let allowance = allowance::project(config, topology, release, binding, usage)
        .map_err(StartupDemandUnavailable::Usage)?;
    if usage.pending_operations > 0 {
        return Err(StartupDemandUnavailable::PendingGrant);
    }
    let balance = balance.ok_or(StartupDemandUnavailable::BalanceNotObserved)?;
    let source = config
        .component_specs
        .get(&binding.component.component_spec)
        .and_then(|source| source.get_canister(&binding.role))
        .ok_or(StartupDemandUnavailable::Usage(
            StartupUsageUnavailable::AuthorityMismatch,
        ))?;
    let threshold = source.topup.as_ref().map(|topup| topup.threshold.to_u128());
    let shortfall = match threshold {
        Some(threshold) => threshold
            .checked_add(1)
            .ok_or(StartupDemandUnavailable::ArithmeticOverflow)?
            .saturating_sub(balance),
        None => 0,
    };
    // Runtime asks for its configured amount at equality, not merely the deficit.
    let next_request = if shortfall == 0 {
        0
    } else {
        source.topup.as_ref().map_or(0, |topup| {
            topup.amount.to_u128().min(
                allowance
                    .next_request_policy_cap_cycles
                    .expect("settled allowance has an exact policy cap"),
            )
        })
    };
    Ok(StartupChildLocalDemand {
        observed_balance_cycles: balance,
        threshold_cycles: threshold,
        shortfall_cycles: shortfall,
        shortfall_beyond_lifetime_allowance_cycles: shortfall
            .saturating_sub(allowance.remaining_after_charges_cycles),
        next_request_policy_cycles: next_request,
    })
}
