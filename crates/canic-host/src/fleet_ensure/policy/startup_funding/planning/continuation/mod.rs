//! Module: fleet_ensure::policy::startup_funding::planning::continuation
//!
//! Responsibility: prepay bounded Root startup demand in the initial continuation review.
//! Does not own: continuation permission, live grants, receipts or Ledger effects.
//! Boundary: raises existing Create/Fund amounts; successor phases remain protocol-only.

use super::{
    CycleBounds, DesiredFleet, DesiredFleetArtifacts, EnsureAction, EnsurePolicyError,
    FleetObservation, PlanAccumulator, canister_cycle_policy, checked_add, fund_root,
};

pub(in crate::fleet_ensure::policy) fn prepay_continuation(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    observation: &FleetObservation,
    bounds: CycleBounds,
    created_at_time: u64,
    accumulator: &mut PlanAccumulator,
) -> Result<(), EnsurePolicyError> {
    for root in
        crate::fleet_ensure::policy::recovery::startup_forecasts(desired, artifacts, bounds)?
    {
        if let Some(shortfall) = root.unfunded_role {
            return Err(EnsurePolicyError::StartupRoleFundingUnavailable {
                root: root.root,
                role: shortfall.role.to_string(),
                shortfall_cycles: shortfall.cycles,
            });
        }
        let minimum = root.required_native_cycles;
        if let Some(live) = observation
            .canisters
            .get(&root.root)
            .and_then(Option::as_ref)
        {
            fund_root(
                desired,
                &root.root,
                live,
                minimum,
                bounds,
                created_at_time,
                accumulator,
            )?;
        } else {
            fund_creation(desired, &root.root, minimum, bounds, accumulator)?;
        }
    }
    Ok(())
}

fn fund_creation(
    desired: &DesiredFleet,
    root: &str,
    minimum: u128,
    bounds: CycleBounds,
    accumulator: &mut PlanAccumulator,
) -> Result<(), EnsurePolicyError> {
    let configured = desired
        .canisters
        .iter()
        .find(|canister| canister.name == root)
        .ok_or_else(|| EnsurePolicyError::MissingObservation {
            name: root.to_owned(),
        })?;
    let minimum = minimum.max(canister_cycle_policy(configured)?.minimum_cycles);
    let planned = accumulator
        .canisters
        .iter_mut()
        .find(|canister| canister.name == root)
        .ok_or_else(|| EnsurePolicyError::MissingObservation {
            name: root.to_owned(),
        })?;
    let margin = bounds
        .update_burn
        .checked_mul(planned.actions.len() as u128)
        .and_then(|updates| updates.checked_add(bounds.observation_burn))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "startup creation margin",
        })?;
    let requested = checked_add(minimum, margin, "startup creation funding")?;
    let initial = planned
        .actions
        .iter_mut()
        .find_map(|action| match action {
            EnsureAction::Create {
                requested_initial_cycles,
                ..
            } => Some(requested_initial_cycles),
            _ => None,
        })
        .ok_or_else(|| EnsurePolicyError::MissingObservation {
            name: root.to_owned(),
        })?;
    let additional = requested.saturating_sub(*initial);
    *initial = (*initial).max(requested);
    // Creation already owns its Ledger fee. Its one reviewed amount includes startup headroom.
    accumulator.add_funding(additional)
}
