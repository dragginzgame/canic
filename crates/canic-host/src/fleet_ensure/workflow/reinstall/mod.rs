//! Module: fleet_ensure::workflow::reinstall
//!
//! Responsibility: recover an unfinished activation under its exact observed authority.
//! Boundary: completed Fleets use current inventory-driven clean reinstall.

pub(super) mod activation;

use super::*;
use crate::fleet_ensure::{model::DesiredFleet, ops::reinstall as capture};

/// Review bounded reset recovery for the selected unfinished activation.
pub fn plan_reinstall<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    desired_sha256: &str,
    requested_fleet: &str,
    created_at_time: u64,
    platform: &mut P,
) -> Result<FleetEnsureReport, EnsureWorkflowError<P::Error>> {
    validate_path_identity(desired, requested_fleet)?;
    crate::fleet_ensure::ops::retained_contract::check(
        root,
        &desired.environment,
        requested_fleet,
    )?;
    let paths = EnsurePaths::under(root, &desired.environment, requested_fleet);
    let _lock = lock_operation(&paths)?;
    verify_release_transition(root, desired, FleetEnsurePlanScope::ReinstallPreparation)?;
    let state = read_state(&paths, requested_fleet)?;
    let source = match capture::source::read(&paths, &desired.environment, requested_fleet) {
        Ok(source) => source,
        Err(EnsureStateError::InvalidActivationSource) => {
            if let Some(journal) = crate::fleet_ensure::ops::read_journal(&paths)?
                && journal.completion == FleetEnsureCompletion::InProgress
                && let Some(plan) = crate::fleet_ensure::ops::read_plan(&paths)?
            {
                verify_journal_integrity(&journal, &plan, requested_fleet, &state)?;
                return Err(EnsureWorkflowError::RetainedOperationRecoveryRequired {
                    operation_id: plan.operation_id,
                    plan_sha256: plan.plan_sha256,
                });
            }
            return Err(EnsureWorkflowError::ReinstallConflict);
        }
        Err(error) => return Err(error.into()),
    };
    activation::plan_preparation(
        root,
        &paths,
        desired,
        desired_sha256,
        created_at_time,
        &source,
        &state,
        platform,
    )
}

const fn report(plan: FleetEnsurePlan) -> FleetEnsureReport {
    FleetEnsureReport {
        funding_review: None,
        actual_conservation: None,
        effects_applied: 0,
        plan,
        terminal: false,
    }
}

pub(super) fn continue_plan<P: EnsurePlatform>(
    root: &Path,
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    prior: &FleetEnsurePlan,
    journal: Option<&FleetEnsureJournalRecord>,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<FleetEnsureReport, EnsureWorkflowError<P::Error>> {
    activation::continue_plan(root, paths, desired, prior, journal, state, platform)
}

pub(super) fn verify_before_apply<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<(FleetObservation, u128), EnsureWorkflowError<P::Error>> {
    activation::verify_before_apply(root, desired, plan, platform, state)
}

pub(super) fn complete<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<ActualCycleConservation, EnsureWorkflowError<P::Error>> {
    activation::complete(plan, journal, state, platform)
}

pub(super) fn verify_terminal_estate<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    observation: &FleetObservation,
) -> Result<(), EnsureWorkflowError<E>> {
    let Some(intent) = plan.reinstall.as_deref() else {
        return Ok(());
    };
    let expected = intent
        .authorities
        .iter()
        .map(|a| a.principal.as_str())
        .chain(intent.assets.iter().map(|a| a.principal.as_str()))
        .collect::<BTreeSet<_>>();
    let observed = observation
        .canisters
        .values()
        .filter_map(|live| live.as_ref().map(|live| live.principal.as_str()))
        .chain(
            observation
                .additional_controlled_cycles
                .keys()
                .map(String::as_str),
        )
        .collect::<BTreeSet<_>>();
    if observed != expected {
        return Err(EnsureWorkflowError::ConvergenceDrift);
    }
    Ok(())
}

pub(super) fn verify_effect_authority<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    action: &EnsureAction,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    let Some(intent) = plan.reinstall.as_deref() else {
        return Ok(());
    };
    if intent.activation_reset.is_some() && plan.scope != FleetEnsurePlanScope::Full {
        return activation::verify_effect_authority(plan, action, state, platform);
    }
    if !matches!(action, EnsureAction::Install { .. }) {
        return Ok(());
    }
    let authorities = platform
        .reinstall_authorities(state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::ReinstallConflict)?;
    let observed = authorities
        .get(action.name())
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let binding = intent
        .authorities
        .iter()
        .find(|binding| binding.name == action.name())
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let current =
        capture::authority_binding(observed).ok_or(EnsureWorkflowError::ConvergenceDrift)?;
    if current != *binding {
        return Err(EnsureWorkflowError::ConvergenceDrift);
    }
    if matches!(
        action,
        EnsureAction::Install {
            canic_init: Some(crate::fleet_ensure::model::DesiredCanisterInit::Root { .. }),
            ..
        }
    ) && !platform
        .reinstall_assets_match(
            intent,
            action.name(),
            crate::fleet_ensure::ops::ReinstallAssetCheck::BeforeReset,
        )
        .map_err(EnsureWorkflowError::Platform)?
    {
        return Err(EnsureWorkflowError::ConvergenceDrift);
    }
    Ok(())
}

/// Verify retained infrastructure and every physical pool controller set at completion.
pub(super) fn verify_terminal_authority<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    let Some(intent) = plan.reinstall.as_deref() else {
        return Ok(());
    };
    let observed = platform
        .reinstall_authorities(state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::ConvergenceDrift)?;
    for binding in &intent.authorities {
        let current = observed
            .get(&binding.name)
            .and_then(capture::authority_binding)
            .ok_or(EnsureWorkflowError::ConvergenceDrift)?;
        let expected = capture::terminal_authority(plan, binding);
        if current != expected {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    let roots = intent
        .assets
        .iter()
        .map(|asset| asset.root.as_str())
        .collect::<BTreeSet<_>>();
    for root in roots {
        if !platform
            .reinstall_assets_match(
                intent,
                root,
                crate::fleet_ensure::ops::ReinstallAssetCheck::Terminal,
            )
            .map_err(EnsureWorkflowError::Platform)?
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    Ok(())
}
