//! Review and finish supplied infrastructure through the ordinary Ensure effect journal.
//!
//! This phase stops before pool handoff; it never reports a completed workload Fleet.

mod automatic;

use crate::fleet_ensure::{
    model::{
        ActualCycleConservation, DesiredFleet, EffectState, EnsureAction, FleetEnsureJournalRecord,
        FleetEnsurePlan, FleetEnsureStateRecord, FleetObservation,
        infrastructure_bootstrap::InfrastructureBootstrapRecord,
    },
    ops::{
        self, EnsurePaths, EnsurePlatform,
        capacity_import::{
            admission::survey::CapacityImportSurveyStore, journal::CapacityImportJournalStore,
        },
        infrastructure_bootstrap as bootstrap,
        infrastructure_bootstrap::InfrastructureBootstrapError,
        infrastructure_bootstrap::survey::BootstrapSurvey,
    },
    view::infrastructure_bootstrap::{
        InfrastructureBootstrapApplyView, InfrastructureBootstrapObservation,
    },
    workflow::{EnsureWorkflowError, ordered_actions, verify_terminal_conservation_with_total},
};
use std::path::Path;

/// Capture original supplied custody with durable, finite status attempts per canister.
/// Repeating the exact completed survey is local and preserves its first successful samples.
pub fn survey(
    root: &Path,
    desired: &DesiredFleet,
    coordinator: crate::fleet_ensure::model::infrastructure_bootstrap::BootstrapCoordinatorSelection,
    declarations_toml: &str,
    icp: &crate::icp::IcpCli,
) -> Result<InfrastructureBootstrapRecord, InfrastructureBootstrapError> {
    crate::fleet_ensure::policy::validate_path_labels(&desired.environment, &desired.fleet)?;
    let paths = EnsurePaths::under(root, &desired.environment, &desired.fleet);
    ops::operation_selection::retirement::prepare_bootstrap(&paths, desired)?;
    let owner = CapacityImportJournalStore::open(&paths)?;
    let mut survey = BootstrapSurvey::begin(&paths, desired, coordinator, declarations_toml)?;
    if let Some(source) = survey.completed() {
        return Ok(source.clone());
    }
    let ids = survey.canisters();
    let mut persistence =
        CapacityImportSurveyStore::open_bootstrap(&owner, &paths, survey.request_sha256(), &ids)?;
    let agent = survey.agent(icp, desired)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut samples = Vec::with_capacity(ids.len());
    for id in ids {
        let sample = if let Some(sample) = persistence.sample(id) {
            sample.clone()
        } else {
            let prepared = runtime.block_on(BootstrapSurvey::prepare_sample(&agent, id))?;
            persistence.reserve(id)?;
            let sample = runtime.block_on(prepared.observe())?;
            persistence.retain(sample.clone())?;
            sample
        };
        samples.push(sample);
    }
    survey.ledger(desired, icp)?;
    runtime.block_on(survey.finish(&paths, desired, &agent, samples))
}

/// Retain a no-replacement initialization review only in a new, untracked Fleet operation.
pub fn plan<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    source: &InfrastructureBootstrapRecord,
    desired_sha256: &str,
    time: u64,
    platform: &mut P,
) -> Result<FleetEnsurePlan, EnsureWorkflowError<P::Error>> {
    let paths = EnsurePaths::under(root, &desired.environment, &desired.fleet);
    let _lock = ops::lock_operation(&paths)?;
    let time =
        bootstrap::inspection::planned_at_time(&paths, source.source_sha256)?.unwrap_or(time);
    let planned = bootstrap::prepare(root, desired, source, desired_sha256, time)?;
    if let Some(retained) = ops::read_plan(&paths)? {
        if retained == planned {
            return Ok(retained);
        }
        return Err(InfrastructureBootstrapError::Integrity.into());
    }
    let state = ops::read_state(&paths, &desired.fleet)?;
    if ops::read_journal(&paths)?.is_some()
        || !state.principals.is_empty()
        || !state.pending_principals.is_empty()
        || !state.topology.is_empty()
        || state.active_registry.is_some()
    {
        return Err(InfrastructureBootstrapError::Integrity.into());
    }
    platform
        .bind_reviewed_desired(desired)
        .map_err(EnsureWorkflowError::Platform)?;
    bootstrap::inspection::reserve(
        &paths,
        &planned,
        bootstrap::inspection::InspectionPhase::Review,
    )?;
    let observed = platform
        .infrastructure_bootstrap_observation(source, &state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    bootstrap::verify_initial(&planned, &observed)?;
    ops::write_plan(&paths, &planned)?;
    Ok(planned)
}

pub(super) fn verify_before_apply<P: EnsurePlatform>(
    root: &Path,
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<(FleetObservation, u128), EnsureWorkflowError<P::Error>> {
    let source = plan
        .infrastructure_bootstrap
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let paths = EnsurePaths::under(root, &plan.environment, &plan.fleet);
    bootstrap::inspection::reserve(&paths, plan, bootstrap::inspection::InspectionPhase::Apply)?;
    let observed = platform
        .infrastructure_bootstrap_observation(source, state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    bootstrap::verify_initial(plan, &observed)?;
    Ok((
        bootstrap::fleet_observation(&observed),
        plan.conservation.observed_controlled_cycles,
    ))
}

pub(super) fn complete<P: EnsurePlatform>(
    root: &Path,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<
    (ActualCycleConservation, InfrastructureBootstrapObservation),
    EnsureWorkflowError<P::Error>,
> {
    let source = plan
        .infrastructure_bootstrap
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let actions = crate::fleet_ensure::workflow::continuation::actions(plan, journal);
    if actions.len() != journal.effects.len()
        || journal.successor_phases.len() != 1
        || !journal.funding_reviews.is_empty()
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    let paths = EnsurePaths::under(root, &plan.environment, &plan.fleet);
    bootstrap::inspection::reserve(
        &paths,
        plan,
        bootstrap::inspection::InspectionPhase::Terminal,
    )?;
    for (action, effect) in actions.iter().zip(&journal.effects) {
        if effect.state != EffectState::Applied
            || effect.action_sha256 != ops::action_sha256(action)
            || (matches!(
                action,
                EnsureAction::Fund { .. } | EnsureAction::Create { .. }
            ) && effect.receipt.is_none())
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        // Applied effects already retained their reconciliation. The final exact
        // custody sample checks their resulting state after later controller changes.
    }
    let observed = platform
        .infrastructure_bootstrap_observation(source, state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let (terminal, total) = bootstrap::terminal_observation(root, plan, state, &observed)?;
    let actual = verify_terminal_conservation_with_total(plan, journal, state, &terminal, total)?;
    Ok((actual, observed))
}

/// Expand the reviewed Store/Registry phase once all infrastructure installations are receipted.
pub(super) fn advance<P: EnsurePlatform>(
    root: &Path,
    plan: &FleetEnsurePlan,
    journal: &mut FleetEnsureJournalRecord,
    state: &mut FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<bool, EnsureWorkflowError<P::Error>> {
    if !journal.successor_phases.is_empty() {
        return Ok(false);
    }
    let actions = ordered_actions(plan);
    if actions.len() != journal.effects.len()
        || journal
            .effects
            .iter()
            .any(|effect| effect.state != EffectState::Applied)
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    let paths = EnsurePaths::under(root, &plan.environment, &plan.fleet);
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    super::publish_terminal_state(desired, plan, journal, state)
        .map_err(|_| EnsureWorkflowError::JournalIntegrity)?;
    ops::write_state(&paths, state)?;

    bootstrap::inspection::reserve(
        &paths,
        plan,
        bootstrap::inspection::InspectionPhase::Registration,
    )?;
    let source = plan
        .infrastructure_bootstrap
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let observed = platform
        .infrastructure_bootstrap_observation(source, state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let (mut observation, total) = bootstrap::terminal_observation(root, plan, state, &observed)?;
    // The shared phase admission counts native and additional controlled balances separately.
    for target in &plan.canisters {
        let sample = observed.canisters[&target.name]
            .as_ref()
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        observation
            .additional_controlled_cycles
            .insert(sample.binding.canister_id.to_text(), sample.reserved_cycles);
    }
    let phase = bootstrap::registration::compile(root, plan, state, total)?;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    crate::fleet_ensure::workflow::continuation::append(
        &paths,
        desired,
        plan,
        journal,
        state,
        &observation,
        phase,
        platform,
    )?;
    Ok(true)
}

/// Review supplied initialization and its exact estate publication through one current operator flow.
pub fn review<P: EnsurePlatform>(
    request: &crate::fleet_ensure::dto::infrastructure_bootstrap::InfrastructureBootstrapReviewRequest<'_>,
    platform: &mut P,
    icp: &crate::icp::IcpCli,
) -> Result<FleetEnsurePlan, EnsureWorkflowError<P::Error>> {
    let source = survey(
        request.workspace,
        request.desired,
        request.coordinator,
        request.declarations_toml,
        icp,
    )?;
    review_source(request, &source, platform)
}

/// Review a normal disposable reset from current authority, without interpreting old records.
pub fn review_clean<P: EnsurePlatform>(
    workspace: &Path,
    desired: &DesiredFleet,
    seed: &Path,
    time: u64,
    platform: &mut P,
    icp: &crate::icp::IcpCli,
) -> Result<FleetEnsurePlan, EnsureWorkflowError<P::Error>> {
    let source = automatic::survey(workspace, desired, icp)?;
    review_source(&crate::fleet_ensure::dto::infrastructure_bootstrap::InfrastructureBootstrapReviewRequest {
        workspace, desired, coordinator: crate::fleet_ensure::model::infrastructure_bootstrap::BootstrapCoordinatorSelection::Initialize,
        declarations_toml: &source.declarations_toml, seed, planned_at_time: time,
    }, &source, platform)
}

fn review_source<P: EnsurePlatform>(
    request: &crate::fleet_ensure::dto::infrastructure_bootstrap::InfrastructureBootstrapReviewRequest<'_>,
    source: &InfrastructureBootstrapRecord,
    platform: &mut P,
) -> Result<FleetEnsurePlan, EnsureWorkflowError<P::Error>> {
    let paths = EnsurePaths::under(
        request.workspace,
        &request.desired.environment,
        &request.desired.fleet,
    );
    let source = bootstrap::publication::bind(&paths, request.desired, source, request.seed)?;
    let desired = bootstrap::publication::bind_holds(request.desired, &source)?;
    let digest = canic_core::cdk::utils::hash::sha256_hex(
        toml::to_string_pretty(&desired)
            .map_err(|_| InfrastructureBootstrapError::Integrity)?
            .as_bytes(),
    );
    let time = ops::read_plan(&paths)?.map_or(request.planned_at_time, |plan| plan.planned_at_time);
    plan(
        request.workspace,
        &desired,
        &source,
        &digest,
        time,
        platform,
    )
}

/// Resolve exact bootstrap authority locally, including an already published operation.
pub fn retained(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    digest: &str,
) -> Result<FleetEnsurePlan, InfrastructureBootstrapError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let plan = match bootstrap::publication::read(&paths, digest)? {
        Some(record) => record.plan,
        None => ops::read_plan(&paths)?.ok_or(InfrastructureBootstrapError::Integrity)?,
    };
    if plan.environment != environment
        || plan.fleet != fleet
        || plan.plan_sha256 != digest
        || plan.scope != crate::fleet_ensure::model::FleetEnsurePlanScope::InfrastructureBootstrap
        || plan.plan_sha256 != crate::fleet_ensure::policy::expected_plan_sha256(&plan)
        || plan
            .infrastructure_bootstrap
            .as_ref()
            .is_none_or(|source| source.estate_seed.is_none())
    {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(plan)
}

/// Resume the reviewed network journal and interruption-safe local estate publication.
pub fn apply<P: EnsurePlatform>(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    digest: &str,
    platform: &mut P,
) -> Result<
    crate::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapPublicationRecord,
    EnsureWorkflowError<P::Error>,
> {
    apply_reporting(workspace, environment, fleet, digest, platform)
        .map(|result| result.publication)
}

/// Preserve the ordinary execution report through local identity publication.
pub(super) fn apply_reporting<P: EnsurePlatform>(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    digest: &str,
    platform: &mut P,
) -> Result<InfrastructureBootstrapApplyView, EnsureWorkflowError<P::Error>> {
    let plan = retained(workspace, environment, fleet, digest)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    if let Some(record) = bootstrap::publication::read(&paths, digest)? {
        if record.completed {
            return Ok(InfrastructureBootstrapApplyView {
                publication: record,
                execution: None,
            });
        }
        return Ok(InfrastructureBootstrapApplyView {
            publication: bootstrap::publication::publish(&paths, &plan)?,
            execution: None,
        });
    }
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    let execution = crate::fleet_ensure::workflow::apply(
        workspace,
        desired,
        &plan.desired_sha256,
        fleet,
        digest,
        platform,
    )?;
    Ok(InfrastructureBootstrapApplyView {
        publication: bootstrap::publication::publish(&paths, &plan)?,
        execution: Some(execution),
    })
}
