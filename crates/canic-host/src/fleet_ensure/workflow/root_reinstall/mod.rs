//! Module: fleet_ensure::workflow::root_reinstall
//!
//! Responsibility: verify the reviewed management reset before and after the journal driver.
//! Does not own: effect dispatch, initialization, or a second recovery journal.
//! Boundary: installed module and controller authority must match the reviewed reset exactly.

use super::{
    EnsureWorkflowError, compatible_root_start_prerequisite, ordered_actions,
    root_management_fleet_observation, verify_terminal_conservation,
};
use crate::fleet_ensure::{
    model::{
        ActualCycleConservation, CanisterRuntimeStatus, DesiredFleet, EffectState, EnsureAction,
        FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsurePlanScope, FleetEnsureStateRecord,
        FleetObservation, InstallMode,
    },
    ops::{EnsurePlatform, action_sha256, resolve_desired_artifacts},
    policy::{RootStartPlanInput, root_reinstall},
};
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
            || (!matches!(action, EnsureAction::Stop { .. })
                && !platform
                    .observe_effect(&plan.operation_id, action, effect, state)
                    .map_err(EnsureWorkflowError::Platform)?
                    .applied)
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
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
    for binding in &plan.root_reinstall_bindings {
        let Some(canister) = plan
            .canisters
            .iter()
            .find(|canister| canister.name == binding.name)
        else {
            return Err(EnsureWorkflowError::PlanIntegrity);
        };
        let [
            EnsureAction::Stop {
                name: stop,
                principal: stop_id,
            },
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
        ] = canister.actions.as_slice()
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
    Ok(targets)
}

/// Recheck controller and module authority immediately before a reset effect.
pub(super) fn verify_effect_authority<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    action: &EnsureAction,
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
    Ok(())
}
