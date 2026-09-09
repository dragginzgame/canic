//! Module: fleet_ensure::workflow::reinstall::activation
//!
//! Responsibility: sequence source-bound preparation through the existing Ensure effect driver.
//! Does not own: physical observation, record conversion, admission policy or another journal.
//! Boundary: preparation settles Roots and reobserves the complete estate before reset review.

use super::*;
use crate::fleet_ensure::{
    model::{DesiredFleet, FleetActivationSourceRecord, FleetReinstallRecord},
    ops::FleetActivationResetObservation,
    policy::{RootStartPlanInput, reinstall::activation as admission},
};

pub(super) fn is_activation_reset(plan: &FleetEnsurePlan) -> bool {
    plan.reinstall
        .as_ref()
        .is_some_and(|intent| intent.activation_reset.is_some())
}

#[expect(
    clippy::too_many_arguments,
    reason = "the locked review binds source evidence, desired input, observed estate and time"
)]
pub(super) fn plan_preparation<P: EnsurePlatform>(
    root: &Path,
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    digest: &str,
    time: u64,
    source: &FleetActivationSourceRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<FleetEnsureReport, EnsureWorkflowError<P::Error>> {
    let observed = inventory(source, state, platform)?;
    let plan = compile(root, desired, digest, time, source, state, &observed)?;
    capture::adoption::stage(paths, &plan)?;
    Ok(super::report(plan))
}

fn compile<E: std::error::Error + 'static>(
    root: &Path,
    desired: &DesiredFleet,
    digest: &str,
    time: u64,
    source: &FleetActivationSourceRecord,
    state: &FleetEnsureStateRecord,
    observed: &FleetActivationResetObservation,
) -> Result<FleetEnsurePlan, EnsureWorkflowError<E>> {
    let retained = capture::capture_imports(desired, &observed.inventory)
        .ok_or(EnsureWorkflowError::ReinstallConflict)?;
    let management = RootManagementObservation {
        roots: observed.inventory.authorities.clone(),
        operator_cycles: 0,
    };
    Ok(admission::preparation(
        admission::ActivationPreparationInput {
            root: RootStartPlanInput {
                authority: None,
                state,
                desired: &retained,
                desired_sha256: digest,
                created_at_time: time,
                requested_fleet: &retained.fleet,
                observation: &management,
            },
            artifacts: &resolve_desired_artifacts(root, &retained)?,
            source,
            roots: &observed.roots,
            assets: &observed.inventory.assets,
            observation: &observed.inventory.observation,
        },
    )?)
}

fn inventory<P: EnsurePlatform>(
    source: &FleetActivationSourceRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<FleetActivationResetObservation, EnsureWorkflowError<P::Error>> {
    platform
        .activation_reset_inventory(source, state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or_else(|| EnsureWorkflowError::PartialActivationResetUnavailable {
            operation_id: source.operation_id.clone(),
            source_document_sha256: source.plan_document_sha256.clone(),
        })
}

pub(super) fn verify_before_apply<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<(FleetObservation, u128), EnsureWorkflowError<P::Error>> {
    if plan.scope == FleetEnsurePlanScope::RootReinstallPrerequisite {
        return verify_reset(root, desired, plan, platform, state);
    }
    if plan.scope == FleetEnsurePlanScope::Full {
        return verify_full(root, desired, plan, platform, state);
    }
    if plan.scope != FleetEnsurePlanScope::ReinstallPreparation {
        return Err(EnsureWorkflowError::ReinstallConflict);
    }
    let intent = intent(plan)?;
    let source = &intent
        .activation_reset
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .source;
    let observed = inventory(source, state, platform)?;
    let current = compile(
        root,
        desired,
        &plan.desired_sha256,
        plan.planned_at_time,
        source,
        state,
        &observed,
    )?;
    if normalized_plan(plan) != normalized_plan(&current) {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok((
        observed.inventory.observation,
        current.conservation.observed_controlled_cycles,
    ))
}

pub(super) fn complete<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<ActualCycleConservation, EnsureWorkflowError<P::Error>> {
    verify_journal(journal, plan, &plan.fleet, state)?;
    if plan.scope != FleetEnsurePlanScope::ReinstallPreparation {
        return Err(EnsureWorkflowError::PlanIntegrity);
    }
    let intent = intent(plan)?;
    let activation = intent
        .activation_reset
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let actions = ordered_actions(plan);
    if actions.len() != journal.effects.len() {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    for (action, effect) in actions.iter().zip(&journal.effects) {
        let followed_by_start = matches!(action, EnsureAction::Stop { .. })
            && actions.iter().any(
                |next| matches!(next, EnsureAction::Start { name, .. } if name == action.name()),
            );
        if effect.state != EffectState::Applied
            || effect.action_sha256 != action_sha256(action)
            || (!followed_by_start
                && !platform
                    .observe_effect(&plan.operation_id, action, effect, state)
                    .map_err(EnsureWorkflowError::Platform)?
                    .applied)
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    let observed = inventory(&activation.source, state, platform)?;
    if observed.inventory.assets != intent.assets || observed.roots != activation.roots {
        return Err(EnsureWorkflowError::ConvergenceDrift);
    }
    verify_authorities(intent, &observed.inventory.authorities)?;
    super::verify_terminal_estate(plan, &observed.inventory.observation)?;
    verify_terminal_conservation(plan, journal, state, &observed.inventory.observation)
}

pub(super) fn verify_effect_authority<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    action: &EnsureAction,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    if plan.scope == FleetEnsurePlanScope::RootReinstallPrerequisite {
        // The ordinary Root reset guard owns module/version checks. A completed stop settles
        // the old Root before install, so controller inspection must occur while it still runs.
        if matches!(action, EnsureAction::Stop { .. }) {
            let activation = intent(plan)?
                .activation_reset
                .as_ref()
                .ok_or(EnsureWorkflowError::PlanIntegrity)?;
            let observed = inventory(&activation.source, state, platform)?;
            let assets_match = observed.inventory.assets == intent(plan)?.assets;
            let roots_match = observed.roots == activation.roots;
            if !assets_match || !roots_match {
                return Err(EnsureWorkflowError::ConvergenceDrift);
            }
        }
        return Ok(());
    }
    if plan.scope != FleetEnsurePlanScope::ReinstallPreparation
        || !matches!(
            action,
            EnsureAction::Stop { .. } | EnsureAction::Start { .. }
        )
    {
        return Err(EnsureWorkflowError::PlanIntegrity);
    }
    let observed = platform
        .reinstall_authorities(state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::ReinstallConflict)?;
    verify_authorities(intent(plan)?, &observed)
}

fn verify_authorities<E: std::error::Error + 'static>(
    intent: &FleetReinstallRecord,
    observed: &BTreeMap<String, crate::fleet_ensure::model::RootManagementCanisterObservation>,
) -> Result<(), EnsureWorkflowError<E>> {
    if observed.len() != intent.authorities.len() {
        return Err(EnsureWorkflowError::ConvergenceDrift);
    }
    for binding in &intent.authorities {
        if observed
            .get(&binding.name)
            .and_then(capture::authority_binding)
            .as_ref()
            != Some(binding)
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    Ok(())
}

fn intent<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
) -> Result<&FleetReinstallRecord, EnsureWorkflowError<E>> {
    plan.reinstall
        .as_deref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)
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
    let Some(journal) = journal.filter(|journal| journal.plan_sha256 == prior.plan_sha256) else {
        return Ok(super::report(prior.clone()));
    };
    let desired = prior
        .reviewed_desired
        .as_deref()
        .map_or(desired, |reviewed| reviewed.desired());
    platform
        .bind_reviewed_desired(desired)
        .map_err(EnsureWorkflowError::Platform)?;
    let next = match (prior.scope, journal.completion) {
        (FleetEnsurePlanScope::ReinstallPreparation, FleetEnsureCompletion::Prepared) => {
            complete(prior, journal, state, platform)?;
            let evidence = capture::adoption::retain_preparation(paths)?;
            reset_plan(root, desired, prior, state, platform, evidence)?
        }
        (FleetEnsurePlanScope::RootReinstallPrerequisite, FleetEnsureCompletion::Converged) => {
            super::super::root_reinstall::complete(prior, journal, state, platform)?;
            let physical = after_reset(prior, state, platform)?;
            let current = capture::capture_after_activation_reset(intent(prior)?, &physical)
                .ok_or(EnsureWorkflowError::ConvergenceDrift)?;
            let observation = platform
                .observe(&prior.operation_id, state)
                .map_err(EnsureWorkflowError::Platform)?;
            compile_plan(
                desired,
                &resolve_desired_artifacts(root, desired)?,
                &[],
                &prior.desired_sha256,
                &prior.fleet,
                &observation,
                prior.planned_at_time,
                &prior.operation_id,
                Some(&current),
            )?
        }
        _ => return Ok(super::report(prior.clone())),
    };
    write_plan(paths, &next)?;
    Ok(super::report(next))
}

fn reset_plan<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    prepared: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
    evidence: crate::fleet_ensure::model::ActivationPreparationEvidenceRecord,
) -> Result<FleetEnsurePlan, EnsureWorkflowError<P::Error>> {
    let activation = intent(prepared)?
        .activation_reset
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let observed = inventory(&activation.source, state, platform)?;
    compile_reset(root, desired, prepared, state, evidence, &observed)
}

fn compile_reset<E: std::error::Error + 'static>(
    root: &Path,
    desired: &DesiredFleet,
    prepared: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    evidence: crate::fleet_ensure::model::ActivationPreparationEvidenceRecord,
    observed: &FleetActivationResetObservation,
) -> Result<FleetEnsurePlan, EnsureWorkflowError<E>> {
    let management = RootManagementObservation {
        roots: observed.inventory.authorities.clone(),
        operator_cycles: 0,
    };
    Ok(admission::reset(
        RootStartPlanInput {
            authority: None,
            state,
            desired,
            desired_sha256: &prepared.desired_sha256,
            created_at_time: prepared.planned_at_time,
            requested_fleet: &prepared.fleet,
            observation: &management,
        },
        &resolve_desired_artifacts(root, desired)?,
        prepared,
        evidence,
        &observed.inventory.observation,
    )?)
}

fn verify_reset<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<(FleetObservation, u128), EnsureWorkflowError<P::Error>> {
    let activation = intent(plan)?
        .activation_reset
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let evidence = activation
        .preparation
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let paths = EnsurePaths::under(root, &plan.environment, &plan.fleet);
    let (prepared, journal) = capture::adoption::read_preparation(&paths, evidence)?;
    let prepared = verified_plan(prepared)?;
    if journal.completion != FleetEnsureCompletion::Prepared
        || prepared.operation_id != plan.operation_id
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    complete(&prepared, &journal, state, platform)?;
    let observed = inventory(&activation.source, state, platform)?;
    let current = compile_reset(root, desired, &prepared, state, evidence.clone(), &observed)?;
    if normalized_plan(plan) != normalized_plan(&current) {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok((
        observed.inventory.observation,
        current.conservation.observed_controlled_cycles,
    ))
}

pub(in crate::fleet_ensure::workflow) fn after_reset<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<crate::fleet_ensure::ops::FleetReinstallObservation, EnsureWorkflowError<P::Error>> {
    let observed = platform
        .activation_reset_inventory_after_reset(intent(plan)?, state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::ConvergenceDrift)?;
    super::verify_terminal_estate(plan, &observed.observation)?;
    for binding in &intent(plan)?.authorities {
        let expected = capture::terminal_authority(plan, binding);
        if observed
            .authorities
            .get(&binding.name)
            .and_then(capture::authority_binding)
            .as_ref()
            != Some(&expected)
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    Ok(observed)
}

fn verify_full<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<(FleetObservation, u128), EnsureWorkflowError<P::Error>> {
    let physical = platform
        .activation_reset_inventory_after_reset(intent(plan)?, state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::ConvergenceDrift)?;
    super::verify_terminal_estate(plan, &physical.observation)?;
    verify_authorities(intent(plan)?, &physical.authorities)?;
    let observation = platform
        .observe(&plan.operation_id, state)
        .map_err(EnsureWorkflowError::Platform)?;
    let current = compile_plan(
        desired,
        &resolve_desired_artifacts(root, desired)?,
        &[],
        &plan.desired_sha256,
        &plan.fleet,
        &observation,
        plan.planned_at_time,
        &plan.operation_id,
        plan.reinstall.as_deref(),
    )?;
    if normalized_plan(plan) != normalized_plan(&current) {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok((observation, current.conservation.observed_controlled_cycles))
}
