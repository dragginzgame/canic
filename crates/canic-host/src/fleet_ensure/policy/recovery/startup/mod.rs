//! Module: fleet_ensure::policy::recovery::startup
//!
//! Responsibility: attribute configuration-bound startup and continuation funding.
//! Does not own: live balance discovery, funding permission or runtime reuse evidence.
//! Boundary: the same conservative demand feeds prepayment and informational forecasts.

use crate::fleet_ensure::{
    model::{
        DesiredFleet, DesiredFleetArtifacts, RootStartupFundingForecast,
        StartupFundingReuseAssumption,
    },
    policy::{
        CycleBounds, EnsurePolicyError, canister_cycle_policy, checked_add,
        recovery::continuation_step_burn,
    },
};

pub(in crate::fleet_ensure::policy) fn startup_forecasts(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    bounds: CycleBounds,
) -> Result<Vec<RootStartupFundingForecast>, EnsurePolicyError> {
    let (Some(bootstrap), Some(protocol)) = (&desired.bootstrap, &desired.protocol) else {
        return Ok(Vec::new());
    };
    let per_step = continuation_step_burn(bounds)?;
    let mut forecasts = Vec::new();
    for root in &bootstrap.roots {
        if !protocol
            .component_group_placements
            .iter()
            .any(|placement| placement.root == root.root)
        {
            continue;
        }
        let demand = artifacts
            .startup_funding_by_root
            .get(&root.root)
            .ok_or_else(|| EnsurePolicyError::MissingArtifactIdentity {
                kind: "startup funding",
                name: root.root.clone(),
            })?;
        if demand.maximum_continuation_steps == 0 {
            return Err(EnsurePolicyError::MissingArtifactIdentity {
                kind: "startup continuation steps",
                name: root.root.clone(),
            });
        }
        let configured = desired
            .canisters
            .iter()
            .find(|canister| canister.name == root.root)
            .ok_or_else(|| EnsurePolicyError::MissingObservation {
                name: root.root.clone(),
            })?;
        let configured_minimum_cycles = canister_cycle_policy(configured)?.minimum_cycles;
        let continuation_allowance_cycles = per_step
            .checked_mul(u128::from(demand.maximum_continuation_steps))
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "startup continuation reserve",
            })?;
        let minimum = checked_add(
            demand.minimum_native_cycles,
            continuation_allowance_cycles,
            "startup continuation minimum",
        )?;
        forecasts.push(RootStartupFundingForecast {
            root: root.root.clone(),
            startup_minimum_cycles: demand.minimum_native_cycles,
            maximum_continuation_steps: demand.maximum_continuation_steps,
            continuation_allowance_cycles,
            configured_minimum_cycles,
            required_native_cycles: minimum.max(configured_minimum_cycles),
            reuse_assumption: StartupFundingReuseAssumption::FreshChildrenAndFullPublication,
            unfunded_role: demand.unfunded_role.clone(),
        });
    }
    Ok(forecasts)
}
