//! Module: fleet_ensure::policy::startup_funding::allowance
//!
//! Responsibility: project exact selected child policy against observed parent-local charges.
//! Boundary: no observation, mutation, burn estimate or authorization of operator spending.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    policy::startup_funding::live_binding,
    view::startup_funding::{
        StartupChildAccounting, StartupChildFundingAllowance, StartupChildFundingBinding,
        StartupUsageUnavailable,
    },
};
use canic_core::{
    bootstrap::compiled::ConfigModel,
    control_plane_support::{
        config::ComponentTopology,
        policy::cycles_funding::{
            FundingLedgerSnapshot, FundingLimits, FundingPolicyViolation,
            cooldown_retry_after_secs, evaluate, remaining_child_budget,
        },
    },
    ids::ReleaseBuildId,
};

/// Bind selected release and Spec policy before interpreting a child's charged usage.
pub(in crate::fleet_ensure) fn project(
    config: &ConfigModel,
    topology: &ComponentTopology,
    release: ReleaseBuildId,
    binding: &StartupChildFundingBinding,
    usage: &StartupChildAccounting,
) -> Result<StartupChildFundingAllowance, StartupUsageUnavailable> {
    live_binding::declared(topology, release, binding)?;
    let source = config
        .component_specs
        .get(&binding.component.component_spec)
        .and_then(|source| source.get_canister(&binding.role))
        .ok_or(StartupUsageUnavailable::AuthorityMismatch)?;
    let limits = FundingLimits {
        max_per_request: source.cycles_funding.max_per_request.to_u128(),
        max_per_child: source.cycles_funding.max_per_child.to_u128(),
        cooldown_secs: source.cycles_funding.cooldown_secs,
    };
    accounting(limits, usage)
}

fn accounting(
    limits: FundingLimits,
    usage: &StartupChildAccounting,
) -> Result<StartupChildFundingAllowance, StartupUsageUnavailable> {
    let now = usage.observed_at_ns / 1_000_000_000;
    if usage.last_accounted_at_secs > now
        || (usage.pending_operations == 0 && usage.reserved_cycles != Some(0))
    {
        return Err(StartupUsageUnavailable::InvalidAccounting);
    }
    let ledger = FundingLedgerSnapshot {
        granted_total: usage.accounted_cycles,
        last_granted_at: usage.last_accounted_at_secs,
    };
    let next_request_policy_cap_cycles = if usage.pending_operations > 0 {
        None
    } else {
        Some(match evaluate(limits, ledger, u128::MAX, now) {
            Ok(decision) => decision.approved_cycles,
            Err(
                FundingPolicyViolation::CooldownActive { .. }
                | FundingPolicyViolation::MaxPerChild { .. },
            ) => 0,
        })
    };
    Ok(StartupChildFundingAllowance {
        maximum_per_child_cycles: limits.max_per_child,
        remaining_after_charges_cycles: remaining_child_budget(limits, ledger),
        cooldown_remaining_secs: cooldown_retry_after_secs(limits, ledger, now).unwrap_or(0),
        next_request_policy_cap_cycles,
    })
}
