//! Module: fleet_ensure::policy::startup_funding::planning
//!
//! Responsibility: fund selected Root startup demand through existing reviewed Fund actions.
//! Does not own: observations, grant issuance, Ledger transport or durable receipts.
//! Boundary: only selected provisioning can retain the extra funding; terminal plans cannot.

mod continuation;
#[cfg(test)]
pub(in crate::fleet_ensure) mod tests;

pub(in crate::fleet_ensure::policy) use continuation::prepay_continuation;

use super::super::{
    CycleBounds, EnsurePolicyError, PlanAccumulator, append_target_funding, canister_cycle_policy,
    checked_add, tranche_protocol_actions,
};
use crate::fleet_ensure::model::{
    CurrentFleetProtocolAction, DesiredCanisterKind, DesiredFleet, DesiredFleetArtifacts,
    EnsureAction, FleetObservation, LiveCanister, StartupFundingRequirement,
};

/// Select protocol work with its startup funding, discarding that funding if provisioning is deferred.
#[expect(
    clippy::too_many_arguments,
    reason = "the tranche compiler shares the existing reviewed planning inputs"
)]
pub(in crate::fleet_ensure::policy) fn tranche(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    observation: &FleetObservation,
    bounds: CycleBounds,
    observed_cycles: u128,
    created_at_time: u64,
    accumulator: &mut PlanAccumulator,
    actions: &mut Vec<EnsureAction>,
) -> Result<(), EnsurePolicyError> {
    let before = has_provisioning(actions).then(|| accumulator.clone());
    if before.is_some() {
        append(
            desired,
            artifacts,
            observation,
            bounds,
            created_at_time,
            accumulator,
            actions,
        )?;
    }
    tranche_protocol_actions(
        desired,
        observation,
        bounds,
        observed_cycles,
        accumulator,
        actions,
    )?;
    if let Some(before) = before
        && !has_provisioning(actions)
    {
        *accumulator = before;
        // The surviving prefix cannot regain the deferred provisioning action.
        tranche_protocol_actions(
            desired,
            observation,
            bounds,
            observed_cycles,
            accumulator,
            actions,
        )?;
    }
    Ok(())
}

fn has_provisioning(actions: &[EnsureAction]) -> bool {
    actions.iter().any(|action| {
        matches!(action,
        EnsureAction::FleetProtocol { action, .. }
        if matches!(action.as_ref(), CurrentFleetProtocolAction::ProvisionComponents { .. }))
    })
}

fn append(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    observation: &FleetObservation,
    bounds: CycleBounds,
    created_at_time: u64,
    accumulator: &mut PlanAccumulator,
    actions: &[EnsureAction],
) -> Result<(), EnsurePolicyError> {
    let Some(bootstrap) = &desired.bootstrap else {
        return Ok(());
    };
    for root in &bootstrap.roots {
        let live = observation
            .canisters
            .get(&root.root)
            .and_then(Option::as_ref)
            .ok_or_else(|| EnsurePolicyError::MissingObservation {
                name: root.root.clone(),
            })?;
        let selected = actions
            .iter()
            .any(|action| selects_root(action, &live.principal));
        if !selected {
            continue;
        }
        let requirement = requirement(artifacts, &root.root)?;
        let protocol_burn = actions.iter().try_fold(0, |total, action| {
            if let EnsureAction::FleetProtocol {
                action,
                principal,
                maximum_execution_burn_cycles,
                ..
            } = action
                && (principal == &live.principal
                    || matches!(
                        action.as_ref(),
                        CurrentFleetProtocolAction::ProvisionComponents { .. }
                    ))
            {
                return checked_add(
                    total,
                    *maximum_execution_burn_cycles,
                    "startup protocol execution reserve",
                );
            }
            Ok(total)
        })?;
        let minimum = checked_add(
            requirement.minimum_native_cycles,
            protocol_burn,
            "startup native requirement",
        )?;
        fund_root(
            desired,
            &root.root,
            live,
            minimum,
            bounds,
            created_at_time,
            accumulator,
        )?;
    }
    Ok(())
}

fn requirement<'a>(
    artifacts: &'a DesiredFleetArtifacts,
    root: &str,
) -> Result<&'a StartupFundingRequirement, EnsurePolicyError> {
    let requirement = artifacts.startup_funding_by_root.get(root).ok_or_else(|| {
        EnsurePolicyError::MissingArtifactIdentity {
            kind: "startup funding",
            name: root.to_owned(),
        }
    })?;
    if let Some(shortfall) = &requirement.unfunded_role {
        return Err(EnsurePolicyError::StartupRoleFundingUnavailable {
            root: root.to_owned(),
            role: shortfall.role.to_string(),
            shortfall_cycles: shortfall.cycles,
        });
    }
    Ok(requirement)
}

fn selects_root(action: &EnsureAction, principal: &str) -> bool {
    let EnsureAction::FleetProtocol { action, .. } = action else {
        return false;
    };
    let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = action.as_ref() else {
        return false;
    };
    request.plan.batches.iter().any(|batch| {
        batch.root.fleet_subnet_root.to_text() == principal && !batch.placements.is_empty()
    })
}

fn fund_root(
    desired: &DesiredFleet,
    name: &str,
    live: &LiveCanister,
    minimum: u128,
    bounds: CycleBounds,
    created_at_time: u64,
    accumulator: &mut PlanAccumulator,
) -> Result<(), EnsurePolicyError> {
    let configured = desired
        .canisters
        .iter()
        .find(|canister| canister.name == name && canister.kind == DesiredCanisterKind::Root)
        .ok_or_else(|| EnsurePolicyError::MissingObservation {
            name: name.to_owned(),
        })?;
    let mut cycle_policy = canister_cycle_policy(configured)?;
    cycle_policy.minimum_cycles = cycle_policy.minimum_cycles.max(minimum);
    let index = accumulator
        .canisters
        .iter()
        .position(|canister| canister.name == name)
        .ok_or_else(|| EnsurePolicyError::MissingObservation {
            name: name.to_owned(),
        })?;
    if let Some(action) = accumulator.canisters[index]
        .actions
        .iter_mut()
        .find(|action| matches!(action, EnsureAction::Fund { .. }))
    {
        let additional = extend_funding(
            action,
            cycle_policy.minimum_cycles.saturating_sub(live.cycles),
        )?;
        accumulator.add_funding(additional)?;
    } else {
        let mut canister_actions = std::mem::take(&mut accumulator.canisters[index].actions);
        let time = created_at_time
            .checked_add(u64::try_from(index).unwrap_or(u64::MAX))
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "startup funding timestamp",
            })?;
        append_target_funding(
            desired,
            configured,
            live,
            cycle_policy,
            bounds,
            time,
            false,
            &mut canister_actions,
            accumulator,
        )?;
        accumulator.canisters[index].actions = canister_actions;
    }
    Ok(())
}

fn extend_funding(action: &mut EnsureAction, deficit: u128) -> Result<u128, EnsurePolicyError> {
    let EnsureAction::Fund {
        amount,
        expected_post_cycles,
        funding_deficit_cycles,
        ..
    } = action
    else {
        return Err(EnsurePolicyError::InvalidProtocolStep(
            action.name().to_owned(),
        ));
    };
    let additional = deficit.saturating_sub(*funding_deficit_cycles);
    *amount = checked_add(*amount, additional, "startup funding amount")?;
    *expected_post_cycles =
        checked_add(*expected_post_cycles, additional, "startup funding balance")?;
    *funding_deficit_cycles = (*funding_deficit_cycles).max(deficit);
    Ok(additional)
}
