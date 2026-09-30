//! Quote exact registration work and target funding without changing original bootstrap authority.

use crate::fleet_ensure::{
    model::{
        EnsureAction, FleetEnsurePlan,
        capacity_import::survey::CapacityImportSampleRecord,
        infrastructure_bootstrap::{
            BOOTSTRAP_EFFECT_INSPECTION_ROUNDS, BOOTSTRAP_EFFECT_OBSERVATIONS_PER_ROUND,
            registration_recovery::RECOVERY_INSPECTION_ROUNDS,
        },
    },
    policy::{
        EnsurePolicyError, canister_cycle_policy, checked_add, cycle_bounds, successor_phase_burn,
    },
};
use std::collections::BTreeMap;

/// Native funding for one exact infrastructure owner, preserving its configured floor.
pub(in crate::fleet_ensure) struct RegistrationTargetFunding {
    pub required: u128,
    pub amount: u128,
    pub deficit: u128,
    pub margin: u128,
}

/// Numeric supplementary authority; workflow must separately review its identity and effects.
pub(in crate::fleet_ensure) struct RegistrationRecoveryQuote {
    pub successor_burn: u128,
    pub recovery_burn: u128,
    pub funding: BTreeMap<String, RegistrationTargetFunding>,
}

/// Attribute each action to its target and divide the shared observation allowance once.
pub(in crate::fleet_ensure) fn quote(
    original: &FleetEnsurePlan,
    phase: &FleetEnsurePlan,
    samples: &BTreeMap<String, CapacityImportSampleRecord>,
) -> Result<RegistrationRecoveryQuote, EnsurePolicyError> {
    let invalid = || {
        EnsurePolicyError::InfrastructureBootstrap(
        crate::fleet_ensure::policy::infrastructure_bootstrap::InfrastructureBootstrapError::Authority,
    )
    };
    let desired = original
        .reviewed_desired
        .as_ref()
        .ok_or_else(invalid)?
        .desired();
    let bounds = cycle_bounds(desired)?;
    let successor_burn = successor_phase_burn(desired, phase)?;
    let mut per_target = original
        .canisters
        .iter()
        .map(|target| (target.name.clone(), 0_u128))
        .collect::<BTreeMap<_, _>>();
    if per_target.is_empty() || per_target.len() != samples.len() {
        return Err(invalid());
    }
    for action in &phase.protocol_actions {
        let EnsureAction::FleetProtocol {
            principal,
            maximum_execution_burn_cycles,
            ..
        } = action
        else {
            return Err(invalid());
        };
        let name = samples
            .iter()
            .find(|(_, sample)| sample.binding.canister_id.to_text() == *principal)
            .map(|(name, _)| name)
            .ok_or_else(invalid)?;
        let attempts = u128::from(action.fixture_publication_attempt_limit().unwrap_or(1));
        let reads = bounds
            .observation_burn
            .checked_mul(3 * attempts)
            .ok_or_else(invalid)?;
        let burn = checked_add(
            *maximum_execution_burn_cycles,
            reads,
            "registration target work",
        )?;
        let total = per_target.get_mut(name).ok_or_else(invalid)?;
        *total = checked_add(*total, burn, "registration target work")?;
    }
    let target_total = per_target.values().try_fold(0, |sum, amount| {
        checked_add(sum, *amount, "registration work")
    })?;
    let shared = successor_burn
        .checked_sub(target_total)
        .ok_or_else(invalid)?;
    let count = per_target.len() as u128;
    let source = original
        .infrastructure_bootstrap
        .as_ref()
        .ok_or_else(invalid)?;
    let inspected = source.sources.len() as u128
        + u128::from(source.coordinator == crate::fleet_ensure::model::infrastructure_bootstrap::BootstrapCoordinatorSelection::Create);
    // Review, approval, registration and terminal each have two durable rounds.
    // A funding effect also retains all eight preparation/reconciliation rounds.
    let reads = (u128::from(RECOVERY_INSPECTION_ROUNDS) * 4 * inspected).div_ceil(count)
        + u128::from(BOOTSTRAP_EFFECT_INSPECTION_ROUNDS)
            * u128::from(BOOTSTRAP_EFFECT_OBSERVATIONS_PER_ROUND)
        + 2;
    let per_owner_recovery = bounds
        .observation_burn
        .checked_mul(reads)
        .and_then(|value| value.checked_add(bounds.update_burn))
        .ok_or_else(invalid)?;
    let recovery_burn = per_owner_recovery.checked_mul(count).ok_or_else(invalid)?;
    let mut funding = BTreeMap::new();
    for (index, (name, burn)) in per_target.into_iter().enumerate() {
        let configured = desired
            .canisters
            .iter()
            .find(|target| target.name == name)
            .ok_or_else(invalid)?;
        let minimum = canister_cycle_policy(configured)?.minimum_cycles;
        let share = shared / count + u128::from((index as u128) < shared % count);
        let minimum = checked_add(
            checked_add(minimum, burn, "registration floor")?,
            share,
            "registration funding",
        )?;
        let sample = samples.get(&name).ok_or_else(invalid)?;
        funding.insert(
            name,
            target_funding(minimum, sample.cycles, per_owner_recovery)?,
        );
    }
    Ok(RegistrationRecoveryQuote {
        successor_burn,
        recovery_burn,
        funding,
    })
}

/// Separate the required native balance from bounded review-to-receipt cycle consumption.
fn target_funding(
    minimum: u128,
    available: u128,
    margin: u128,
) -> Result<RegistrationTargetFunding, EnsurePolicyError> {
    let required = checked_add(minimum, margin, "registration inspection reserve")?;
    let deficit = if available < required {
        minimum.saturating_sub(available).max(1)
    } else {
        0
    };
    let margin = if deficit > 0 { margin } else { 0 };
    let amount = checked_add(deficit, margin, "registration funding amount")?;
    Ok(RegistrationTargetFunding {
        required: checked_add(available, amount, "registration expected balance")?,
        amount,
        deficit,
        margin,
    })
}
