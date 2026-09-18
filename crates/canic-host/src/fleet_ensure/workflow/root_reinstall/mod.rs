//! Module: fleet_ensure::workflow::root_reinstall
//!
//! Responsibility: verify the reviewed management reset before and after the journal driver.
//! Does not own: effect dispatch, initialization, or a second recovery journal.
//! Boundary: installed module and controller authority must match the reviewed reset exactly.

#[cfg(test)]
mod tests;

use super::{
    EnsureWorkflowError, compatible_root_start_prerequisite, ordered_actions,
    root_management_fleet_observation, verify_terminal_conservation,
};
use crate::fleet_ensure::{
    model::{
        ActualCycleConservation, CanisterPlan, CanisterRuntimeStatus, DesiredFleet, EffectState,
        EnsureAction, FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsurePlanScope,
        FleetEnsureStateRecord, FleetObservation, InstallMode, RootManagementBinding,
    },
    ops::{
        EnsurePlatform, NativeFundingObservation, action_sha256, native_funding_applied,
        resolve_desired_artifacts,
    },
    policy::{RootStartPlanInput, root_reinstall},
};
use canic_core::cdk::types::Cycles;
use std::{collections::BTreeSet, path::Path};

pub(super) fn verify_before_apply<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<(FleetObservation, u128), EnsureWorkflowError<P::Error>> {
    let targets = targets(plan)?;
    let management = platform
        .observe_root_management(state, &targets)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let current = root_reinstall::compile(
        RootStartPlanInput {
            state,
            authority: None,
            created_at_time: plan.planned_at_time,
            desired,
            desired_sha256: &plan.desired_sha256,
            observation: &management,
            requested_fleet: &plan.fleet,
        },
        &resolve_desired_artifacts(root, desired)?,
    )?
    .ok_or(EnsureWorkflowError::DriftedBeforeApply)?;
    if !compatible_root_start_prerequisite(plan, &current, desired) {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    let observation = root_management_fleet_observation(&management, &targets)?;
    Ok((observation, current.conservation.observed_controlled_cycles))
}

pub(super) fn complete<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<ActualCycleConservation, EnsureWorkflowError<P::Error>> {
    let targets = targets(plan)?;
    let actions = ordered_actions(plan);
    if actions.len() != journal.effects.len() || !journal.successor_phases.is_empty() {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    for (action, effect) in actions.iter().zip(&journal.effects) {
        if effect.state != EffectState::Applied
            || effect.action_sha256 != action_sha256(action)
            || (!matches!(
                action,
                EnsureAction::Stop { .. } | EnsureAction::Fund { .. }
            ) && !platform
                .observe_effect(&plan.operation_id, action, effect, state)
                .map_err(EnsureWorkflowError::Platform)?
                .applied)
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
        if let EnsureAction::Fund {
            amount,
            expected_post_cycles,
            funding_deficit_cycles,
            funding_margin_cycles,
            ..
        } = action
            && (effect.receipt.is_none()
                || !native_funding_applied(NativeFundingObservation {
                    amount: *amount,
                    expected_post_cycles: *expected_post_cycles,
                    funding_deficit_cycles: *funding_deficit_cycles,
                    funding_margin_cycles: *funding_margin_cycles,
                    pre_cycles: effect.pre_cycles,
                    live_cycles: effect.post_cycles,
                }))
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
    }
    let management = platform
        .observe_root_management(state, &targets)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    for binding in &plan.root_reinstall_bindings {
        let observed = management
            .roots
            .get(&binding.name)
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        let mut controllers = observed.live.controllers.clone();
        controllers.sort();
        if observed.live.principal != binding.principal
            || observed.subnet != binding.subnet
            || controllers != binding.controllers
            || observed.live.status != CanisterRuntimeStatus::Running
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    let terminal = if plan
        .reinstall
        .as_ref()
        .is_some_and(|intent| intent.activation_reset.is_some())
    {
        super::reinstall::activation::after_reset(plan, state, platform)?.observation
    } else {
        root_management_fleet_observation(&management, &targets)?
    };
    verify_terminal_conservation(plan, journal, state, &terminal)
}

fn targets<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
) -> Result<BTreeSet<String>, EnsureWorkflowError<E>> {
    if plan.scope != FleetEnsurePlanScope::RootReinstallPrerequisite
        || plan.root_start_authority.is_some()
        || plan.continuation.is_some()
        || !plan.protocol_actions.is_empty()
        || plan.root_reinstall_bindings.is_empty()
        || plan.canisters.len() != plan.root_reinstall_bindings.len()
    {
        return Err(EnsureWorkflowError::PlanIntegrity);
    }
    let mut targets = BTreeSet::new();
    let mut funding = 0_u128;
    let mut fees = 0_u128;
    for binding in &plan.root_reinstall_bindings {
        let Some(canister) = plan
            .canisters
            .iter()
            .find(|canister| canister.name == binding.name)
        else {
            return Err(EnsureWorkflowError::PlanIntegrity);
        };
        let (stop, remaining) = canister
            .actions
            .split_first()
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        let EnsureAction::Stop {
            name: stop,
            principal: stop_id,
        } = stop
        else {
            return Err(EnsureWorkflowError::PlanIntegrity);
        };
        let remaining =
            if let [action @ EnsureAction::Fund { amount, .. }, remaining @ ..] = remaining {
                let fee = verify_payment_plan(plan, canister, binding, action)?;
                funding = funding
                    .checked_add(*amount)
                    .ok_or(EnsureWorkflowError::PlanIntegrity)?;
                fees = fees
                    .checked_add(fee)
                    .ok_or(EnsureWorkflowError::PlanIntegrity)?;
                remaining
            } else {
                remaining
            };
        let [
            EnsureAction::Install {
                name,
                principal,
                mode: InstallMode::Reinstall,
                ..
            },
            EnsureAction::Start {
                name: start,
                principal: start_id,
            },
        ] = remaining
        else {
            return Err(EnsureWorkflowError::PlanIntegrity);
        };
        let expected = (&binding.name, &binding.principal);
        if !targets.insert(binding.name.clone())
            || [(stop, stop_id), (name, principal), (start, start_id)]
                .into_iter()
                .any(|observed| observed != expected)
        {
            return Err(EnsureWorkflowError::PlanIntegrity);
        }
    }
    if plan.conservation.maximum_new_funding_cycles != funding
        || plan.conservation.maximum_unavoidable_fee_cycles != fees
        || funding.checked_add(fees) != Some(plan.conservation.maximum_operator_debit_cycles)
    {
        return Err(EnsureWorkflowError::PlanIntegrity);
    }
    Ok(targets)
}

/// Check payment identity and budget independently of current live balances.
fn verify_payment_plan<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    canister: &CanisterPlan,
    binding: &RootManagementBinding,
    action: &EnsureAction,
) -> Result<u128, EnsureWorkflowError<E>> {
    let EnsureAction::Fund {
        name,
        principal,
        ledger,
        created_at_time,
        pool_funding: None,
        amount,
        expected_post_cycles,
        funding_deficit_cycles,
        funding_margin_cycles,
    } = action
    else {
        return Err(EnsureWorkflowError::PlanIntegrity);
    };
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    let exact_payment = (name, principal, ledger, *created_at_time)
        == (
            &binding.name,
            &binding.principal,
            &desired.cycles_ledger,
            plan.planned_at_time,
        );
    let exact_amount = *funding_deficit_cycles > 0
        && funding_deficit_cycles.checked_add(*funding_margin_cycles) == Some(*amount)
        && expected_post_cycles.checked_sub(*amount) == Some(canister.observed_cycles);
    if !exact_payment || !exact_amount {
        return Err(EnsureWorkflowError::PlanIntegrity);
    }
    desired
        .ledger_fee_cycles
        .parse::<Cycles>()
        .map(|fee| fee.to_u128())
        .map_err(|_| EnsureWorkflowError::PlanIntegrity)
}

/// Recheck controller and module authority immediately before a reset effect.
pub(super) fn verify_effect_authority<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    action: &EnsureAction,
    pre_cycles: Option<u128>,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    let targets = targets(plan)?;
    let management = platform
        .observe_root_management(state, &targets)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let binding = plan
        .root_reinstall_bindings
        .iter()
        .find(|binding| binding.name == action.name())
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let live = management
        .roots
        .get(&binding.name)
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let mut actual = binding.clone();
    actual.controllers.clone_from(&live.live.controllers);
    actual.controllers.sort();
    actual.name.clone_from(&live.name);
    actual.principal.clone_from(&live.live.principal);
    actual.subnet.clone_from(&live.subnet);
    actual.module_sha256 = live
        .live
        .module_sha256
        .clone()
        .ok_or(EnsureWorkflowError::ConvergenceDrift)?;
    let mut expected = binding.clone();
    if matches!(action, EnsureAction::Start { .. }) {
        let install = plan
            .canisters
            .iter()
            .find(|canister| canister.name == binding.name)
            .and_then(|canister| {
                canister.actions.iter().find_map(|action| match action {
                    EnsureAction::Install { wasm_sha256, .. } => Some(wasm_sha256),
                    _ => None,
                })
            })
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        expected.module_sha256.clone_from(install);
    }
    if actual != expected {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    if matches!(
        action,
        EnsureAction::Fund { .. } | EnsureAction::Install { .. }
    ) && live.live.status != CanisterRuntimeStatus::Stopped
    {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    if let EnsureAction::Fund {
        amount,
        expected_post_cycles,
        funding_margin_cycles,
        ..
    } = action
    {
        let reviewed_pre = expected_post_cycles
            .checked_sub(*amount)
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        let retained_pre = pre_cycles.ok_or(EnsureWorkflowError::JournalIntegrity)?;
        if reviewed_pre.saturating_sub(retained_pre) > *funding_margin_cycles {
            return Err(EnsureWorkflowError::DriftedBeforeApply);
        }
        let minimum = expected_post_cycles
            .checked_sub(*funding_margin_cycles)
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        if live
            .live
            .cycles
            .checked_add(*amount)
            .is_none_or(|funded| funded < minimum)
        {
            return Err(EnsureWorkflowError::DriftedBeforeApply);
        }
        // A retained intent retries the exact Ledger identity, even after a lost reply.
        // Do not require the pre-payment balance again: the first withdrawal may have paid.
        operator_funding(plan, platform)?;
    }
    Ok(())
}

/// Verify the whole reviewed debit before stopping any old runtime.
pub(super) fn verify_initial_funding<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    initial_operator_cycles: u128,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    if plan.scope != FleetEnsurePlanScope::RootReinstallPrerequisite {
        return Ok(());
    }
    targets::<P::Error>(plan)?;
    if plan.conservation.maximum_new_funding_cycles == 0 {
        return Ok(());
    }
    let observed = operator_funding(plan, platform)?;
    if observed.operator_cycles != initial_operator_cycles {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    let required = plan.conservation.maximum_operator_debit_cycles;
    if observed.operator_cycles < required {
        return Err(EnsureWorkflowError::InsufficientOperatorCycles {
            actual: observed.operator_cycles,
            required,
        });
    }
    Ok(())
}

fn operator_funding<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    platform: &mut P,
) -> Result<crate::fleet_ensure::view::OperatorFundingObservation, EnsureWorkflowError<P::Error>> {
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    let fee = desired
        .ledger_fee_cycles
        .parse::<Cycles>()
        .map_err(|_| EnsureWorkflowError::PlanIntegrity)?
        .to_u128();
    let observed = platform
        .observe_operator_funding()
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    if observed.cycles_ledger != desired.cycles_ledger || observed.ledger_fee_cycles != fee {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok(observed)
}
