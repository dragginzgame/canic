//! Module: fleet_ensure::workflow::reinstall
//!
//! Responsibility: sequence explicit reset preparation and its complete inventory review.
//! Does not own: management effects, import conversion or a separate journal.
//! Boundary: completed Fleets and source-bound partial activations use distinct preparation reviews.

pub(super) mod activation;
mod terminal;
#[cfg(test)]
mod tests;

use super::*;
use crate::fleet_ensure::{
    model::{
        CanisterRuntimeStatus, DesiredFleet, FleetReinstallRecord, FleetReinstallSourceRecord,
    },
    ops::reinstall as capture,
    policy::reinstall as policy,
};

/// Plan a new deliberate wipe selecting a build for the currently converged Fleet.
pub fn plan_reinstall<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    desired_sha256: &str,
    requested_fleet: &str,
    created_at_time: u64,
    platform: &mut P,
) -> Result<FleetEnsureReport, EnsureWorkflowError<P::Error>> {
    validate_path_identity(desired, requested_fleet)?;
    let paths = EnsurePaths::under(root, &desired.environment, requested_fleet);
    let _lock = lock_operation(&paths)?;
    let state = read_state(&paths, requested_fleet)?;
    // Source evidence is not an executable journal. Inspect it before requiring
    // current effect fields; the existing review still owns live reset admission.
    match capture::source::read(&paths, &desired.environment, requested_fleet) {
        Ok(source) => {
            return activation::plan_preparation(
                root,
                &paths,
                desired,
                desired_sha256,
                created_at_time,
                &source,
                &state,
                platform,
            );
        }
        Err(EnsureStateError::InvalidActivationSource) => {}
        Err(error) => return Err(error.into()),
    }
    let journal = match retained_plan::journal(&paths, &desired.environment, requested_fleet) {
        Ok(Some(journal)) => journal,
        Ok(None) => return Err(EnsureWorkflowError::JournalIntegrity),
        Err(EnsureWorkflowError::RetainedTerminalReviewRequired { .. }) => {
            return terminal::plan_preparation(
                root,
                &paths,
                desired,
                desired_sha256,
                created_at_time,
                &state,
                platform,
            );
        }
        Err(error) => return Err(error),
    };
    let prior = match read_plan(&paths) {
        Ok(Some(plan)) => verified_plan(plan)?,
        Ok(None) => return Err(EnsureWorkflowError::PlanMissing),
        Err(EnsureStateError::Decode { .. }) => {
            return terminal::plan_preparation(
                root,
                &paths,
                desired,
                desired_sha256,
                created_at_time,
                &state,
                platform,
            );
        }
        Err(error) => return Err(error.into()),
    };
    if journal.completion == FleetEnsureCompletion::InProgress {
        verify_journal_integrity(&journal, &prior, requested_fleet, &state)?;
        return Err(EnsureWorkflowError::RetainedOperationRecoveryRequired {
            operation_id: prior.operation_id,
            plan_sha256: prior.plan_sha256,
        });
    }
    if prior.scope != FleetEnsurePlanScope::Full
        || journal.completion != FleetEnsureCompletion::Converged
        || journal.plan_sha256 != prior.plan_sha256
    {
        return Err(EnsureWorkflowError::ReinstallConflict);
    }
    verify_journal(&journal, &prior, requested_fleet, &state)?;
    if state.active_registry.is_none() {
        return Err(EnsureWorkflowError::ReinstallConflict);
    }
    let source = completed_inventory_operation(&prior, &journal, &state)?;
    let source_desired = prior
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    let source_authority = capture::capture_source(root, source_desired)?;
    observe_source(&source_authority, desired, platform, |platform| {
        platform.terminal_inventory(source, &state)
    })?;
    let operation = canic_core::cdk::utils::hash::sha256_hex(
        format!(
            "canic-fleet-reinstall:{desired_sha256}:{}:{created_at_time}",
            prior.operation_id
        )
        .as_bytes(),
    );
    let plan = preparation(
        root,
        desired,
        desired_sha256,
        source,
        &source_authority,
        &operation,
        created_at_time,
        platform,
        &state,
    )?;
    write_plan(&paths, &plan)?;
    Ok(report(plan))
}

fn source_authority<E: std::error::Error + 'static>(
    intent: &FleetReinstallRecord,
) -> Result<&FleetReinstallSourceRecord, EnsureWorkflowError<E>> {
    intent
        .source
        .as_deref()
        .filter(|_| intent.activation_reset.is_none())
        .ok_or(EnsureWorkflowError::PlanIntegrity)
}

fn verify_selected_artifacts<E: std::error::Error + 'static>(
    root: &Path,
    desired: &DesiredFleet,
    intent: &FleetReinstallRecord,
) -> Result<(), EnsureWorkflowError<E>> {
    let current = capture::target_artifacts_sha256(root, desired)?;
    if intent.target_artifacts_sha256.as_deref() != Some(&current) {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok(())
}

/// Restore the selected target even when a source observation fails.
fn observe_source<P: EnsurePlatform, T>(
    source: &FleetReinstallSourceRecord,
    target: &DesiredFleet,
    platform: &mut P,
    observe: impl FnOnce(&mut P) -> Result<T, P::Error>,
) -> Result<T, EnsureWorkflowError<P::Error>> {
    platform
        .bind_reviewed_desired(source.reviewed_desired.desired())
        .map_err(EnsureWorkflowError::Platform)?;
    let result = observe(platform);
    platform
        .bind_reviewed_desired(target)
        .map_err(EnsureWorkflowError::Platform)?;
    result.map_err(EnsureWorkflowError::Platform)
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

#[expect(
    clippy::too_many_arguments,
    reason = "preparation binds the current Fleet and the new operation identity"
)]
fn preparation<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    digest: &str,
    source: &str,
    source_authority: &FleetReinstallSourceRecord,
    operation: &str,
    time: u64,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<FleetEnsurePlan, EnsureWorkflowError<P::Error>> {
    if capture::refresh_source(root, source_authority)? != *source_authority {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    let target_artifacts_sha256 = capture::target_artifacts_sha256(root, desired)?;
    let roots = observe_source(source_authority, desired, platform, |platform| {
        platform.reinstall_authorities(state)
    })?
    .ok_or(EnsureWorkflowError::ReinstallConflict)?;
    let observation = RootManagementObservation {
        roots,
        operator_cycles: 0,
    };
    Ok(policy::preparation(policy::PreparationInput {
        desired,
        source: source_authority,
        target_artifacts_sha256: &target_artifacts_sha256,
        observation: &observation,
        source_operation_id: source,
        desired_sha256: digest,
        operation_id: operation,
        time,
    })?)
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
    if activation::is_activation_reset(prior) {
        return activation::continue_plan(root, paths, desired, prior, journal, state, platform);
    }
    let Some(journal) = journal.filter(|journal| {
        journal.plan_sha256 == prior.plan_sha256
            && journal.completion == FleetEnsureCompletion::Prepared
    }) else {
        return Ok(report(prior.clone()));
    };
    complete(prior, journal, state, platform)?;
    let intent = prior
        .reinstall
        .as_deref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    verify_selected_artifacts(root, desired, intent)?;
    let inventory = observe_source(source_authority(intent)?, desired, platform, |platform| {
        platform.reinstall_inventory(&intent.source_operation_id, state)
    })?
    .ok_or(EnsureWorkflowError::ReinstallConflict)?;
    let retained = capture::capture_imports(desired, &inventory)
        .ok_or(EnsureWorkflowError::ReinstallConflict)?;
    let intent = capture::capture_intent(intent, &inventory);
    platform
        .bind_reviewed_desired(&retained)
        .map_err(EnsureWorkflowError::Platform)?;
    let plan = compile_plan(
        &retained,
        &resolve_desired_artifacts(root, &retained)?,
        &[],
        &prior.desired_sha256,
        &prior.fleet,
        &inventory.observation,
        prior.planned_at_time,
        &intent.operation_id,
        Some(&intent),
    )?;
    if plan.continuation.is_none() {
        return Err(EnsureWorkflowError::ReinstallConflict);
    }
    write_plan(paths, &plan)?;
    Ok(report(plan))
}

pub(super) fn verify_before_apply<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<(FleetObservation, u128), EnsureWorkflowError<P::Error>> {
    if activation::is_activation_reset(plan) {
        return activation::verify_before_apply(root, desired, plan, platform, state);
    }
    let intent = plan
        .reinstall
        .as_deref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let (current, observation) = if plan.scope == FleetEnsurePlanScope::ReinstallPreparation {
        if let Some(evidence) = &source_authority(intent)?.terminal_retirement {
            let paths = EnsurePaths::under(root, &desired.environment, &desired.fleet);
            let source = capture::terminal::read(&paths, &desired.environment, &desired.fleet)?;
            if source.documents != evidence.source {
                return Err(EnsureWorkflowError::DriftedBeforeApply);
            }
            terminal::verify(desired, source_authority(intent)?, &source, state, platform)?;
        }
        let current = preparation(
            root,
            desired,
            &plan.desired_sha256,
            &intent.source_operation_id,
            source_authority(intent)?,
            &plan.operation_id,
            plan.planned_at_time,
            platform,
            state,
        )?;
        let observation = seal_observation(plan, state, platform)?;
        (current, observation)
    } else {
        verify_seals(root, desired, plan, intent, state, platform)?;
        verify_selected_artifacts(root, desired, intent)?;
        let inventory = observe_source(source_authority(intent)?, desired, platform, |platform| {
            platform.reinstall_inventory(&intent.source_operation_id, state)
        })?
        .ok_or(EnsureWorkflowError::ReinstallConflict)?;
        if inventory.assets != intent.assets {
            return Err(EnsureWorkflowError::DriftedBeforeApply);
        }
        let current = compile_plan(
            desired,
            &resolve_desired_artifacts(root, desired)?,
            &[],
            &plan.desired_sha256,
            &plan.fleet,
            &inventory.observation,
            plan.planned_at_time,
            &plan.operation_id,
            Some(intent),
        )?;
        (current, inventory.observation)
    };
    if normalized_plan(plan) != normalized_plan(&current) {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok((observation, current.conservation.observed_controlled_cycles))
}

fn verify_seals<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
    intent: &FleetReinstallRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    let preparation = preparation(
        root,
        desired,
        &plan.desired_sha256,
        &intent.source_operation_id,
        source_authority(intent)?,
        &plan.operation_id,
        plan.planned_at_time,
        platform,
        state,
    )?;
    for action in ordered_actions(&preparation) {
        if !platform
            .authority_sealed(&plan.operation_id, action)
            .map_err(EnsureWorkflowError::Platform)?
        {
            return Err(EnsureWorkflowError::DriftedBeforeApply);
        }
    }
    Ok(())
}

fn seal_observation<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<FleetObservation, EnsureWorkflowError<P::Error>> {
    let authorities = platform
        .reinstall_authorities(state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::ReinstallConflict)?;
    let targets = plan.canisters.iter().map(|c| c.name.clone()).collect();
    root_management_fleet_observation(
        &RootManagementObservation {
            roots: authorities,
            operator_cycles: 0,
        },
        &targets,
    )
}

pub(super) fn complete<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<ActualCycleConservation, EnsureWorkflowError<P::Error>> {
    if activation::is_activation_reset(plan) {
        return activation::complete(plan, journal, state, platform);
    }
    verify_journal(journal, plan, &plan.fleet, state)?;
    let actions = ordered_actions(plan);
    if actions.len() != journal.effects.len() {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    for (action, effect) in actions.iter().zip(&journal.effects) {
        if !matches!(action, EnsureAction::SealAuthority { .. })
            || effect.state != EffectState::Applied
            || !platform
                .observe_effect(&plan.operation_id, action, effect, state)
                .map_err(EnsureWorkflowError::Platform)?
                .applied
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    verify_terminal_conservation(
        plan,
        journal,
        state,
        &seal_observation(plan, state, platform)?,
    )
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
    let funding_kind = policy::funding_authority_kind(intent, action, &plan.canisters);
    if !matches!(
        action,
        EnsureAction::SealAuthority { .. } | EnsureAction::Install { .. }
    ) && funding_kind.is_none()
    {
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
    if let Some(kind) = funding_kind {
        let (EnsureAction::Fund { principal, .. } | EnsureAction::FundEstate { principal, .. }) =
            action
        else {
            return Err(EnsureWorkflowError::PlanIntegrity);
        };
        if principal != &binding.principal || observed.live.status != CanisterRuntimeStatus::Running
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
        let seal = capture::funding_seal(source_authority(intent)?, binding, kind)
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        if !platform
            .authority_sealed(&plan.operation_id, &seal)
            .map_err(EnsureWorkflowError::Platform)?
        {
            return Err(EnsureWorkflowError::DriftedBeforeApply);
        }
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
