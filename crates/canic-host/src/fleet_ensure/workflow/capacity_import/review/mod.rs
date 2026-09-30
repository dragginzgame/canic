//! Connect completed current authority, durable initial observations and exact operator approval.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        dto::capacity_import::CapacityImportReviewRequest,
        model::{
            EffectState, FleetEnsureCompletion, FleetEnsurePlan, FleetEnsurePlanScope,
            FleetEnsureStateRecord, capacity_import::CapacityImportJournalRecord,
        },
        ops::{
            self, EnsurePaths,
            capacity_import::{
                admission::{
                    observer::CapacityImportLiveObserver, review::ReviewSurvey,
                    survey::CapacityImportSurveyStore,
                },
                journal::{self, CapacityImportJournalError, CapacityImportJournalStore},
                publication,
            },
        },
        policy::validate_path_labels,
        workflow::{continuation, verified_plan, verify_journal},
    },
    icp::IcpCli,
};
use std::{io, path::Path};

/// Review a current Fleet using the host-owned async runtime.
pub fn plan(
    workspace: &Path,
    request: &CapacityImportReviewRequest,
    icp: &IcpCli,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    runtime()?.block_on(plan_async(workspace, request, icp))
}

/// Resume exact reviewed authority, including effect-free terminal replay.
pub fn apply(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    review_sha256: [u8; 32],
    icp: &IcpCli,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    runtime()?.block_on(apply_async(
        workspace,
        environment,
        fleet,
        review_sha256,
        icp,
    ))
}

fn runtime() -> Result<tokio::runtime::Runtime, io::Error> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

/// Plan enrollment after completed Fleet or bootstrap setup; initial status calls have durable bounds.
async fn plan_async(
    workspace: &Path,
    request: &CapacityImportReviewRequest,
    icp: &IcpCli,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    let paths = paths(workspace, &request.environment, &request.fleet)?;
    let store = CapacityImportJournalStore::open(&paths)?;
    if store
        .read()?
        .is_some_and(|record| record.approved && !publication::completed(&record))
    {
        return Err(CapacityImportJournalError::Conflict);
    }
    ops::retained_contract::check(workspace, &request.environment, &request.fleet)
        .map_err(|_| CapacityImportJournalError::InfrastructureRequired)?;
    let (completed, state) = completed(&paths, &request.environment, &request.fleet)?;
    let desired = completed
        .reviewed_desired
        .as_ref()
        .ok_or(CapacityImportJournalError::InfrastructureRequired)?
        .desired();
    let survey = ReviewSurvey::inspect(&paths, request, desired, &state, icp).await?;
    let bootstrap = completed.infrastructure_bootstrap.as_deref();
    let bootstrap_terminal = if let Some(source) = bootstrap {
        survey.require_bootstrap_sources(source, request)?;
        let journal = ops::read_journal(&paths)?.ok_or(CapacityImportJournalError::Integrity)?;
        Some(
            ops::infrastructure_bootstrap::terminal::read_receipt(
                &paths, &completed, &journal, &state,
            )
            .map_err(|_| CapacityImportJournalError::Integrity)?
            .ok_or(CapacityImportJournalError::Integrity)?,
        )
    } else {
        None
    };

    let canisters = survey.canisters(request);
    let mut persistence =
        CapacityImportSurveyStore::open(&store, &paths, survey.request_sha256(), &canisters)?;
    let mut samples = Vec::with_capacity(canisters.len());
    for canister in canisters {
        let original = bootstrap
            .and_then(|source| {
                source
                    .sources
                    .values()
                    .find(|source| source.sample.binding.canister_id == canister)
            })
            .filter(|_| canister != survey.root())
            .map(|source| &source.sample);
        let prior_root = bootstrap_terminal.as_ref().and_then(|receipt| {
            receipt
                .canisters
                .values()
                .find(|sample| sample.binding.canister_id == canister)
        });
        let sample = if let Some(sample) = original
            .or(prior_root)
            .or_else(|| persistence.sample(canister))
        {
            sample.clone()
        } else {
            let prepared = survey.prepare_sample(canister).await?;
            persistence.reserve(canister)?;
            let sample = prepared.observe().await?;
            persistence.retain(sample.clone())?;
            sample
        };
        samples.push(sample);
    }
    let plan = survey.finish(request, &samples).await?;
    let reviewed = journal::reviewed(plan)?;
    let review = if bootstrap.is_some() {
        publication::bind_initial(&paths, &reviewed, &request.policy, &request.seed)?
    } else {
        publication::bind(&paths, &reviewed, &request.policy, &request.seed)?
    };
    store.stage_review(review)
}

/// Apply retained authority, returning completed current or archived receipts before resolving ICP.
async fn apply_async(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    review_sha256: [u8; 32],
    icp: &IcpCli,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    let paths = paths(workspace, environment, fleet)?;
    let store = CapacityImportJournalStore::open(&paths)?;
    if let Some(completed) = store.completed_review(review_sha256)? {
        return Ok(completed);
    }
    let record = store.read()?.ok_or(CapacityImportJournalError::Integrity)?;
    if record
        .operation
        .as_ref()
        .is_none_or(|operation| operation.review.review_sha256 != review_sha256)
    {
        return Err(CapacityImportJournalError::PublicationConflict);
    }
    let mut observer = CapacityImportLiveObserver::from_icp(icp)?;
    crate::fleet_ensure::workflow::capacity_import::apply(
        &store,
        &paths,
        review_sha256,
        icp,
        &mut observer,
    )
    .await
}

fn paths(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<EnsurePaths, CapacityImportJournalError> {
    validate_path_labels(environment, fleet).map_err(|_| CapacityImportJournalError::Integrity)?;
    Ok(EnsurePaths::under(workspace, environment, fleet))
}

fn completed(
    paths: &EnsurePaths,
    environment: &str,
    fleet: &str,
) -> Result<(FleetEnsurePlan, FleetEnsureStateRecord), CapacityImportJournalError> {
    let missing = || CapacityImportJournalError::InfrastructureRequired;
    let plan = verified_plan::<io::Error>(ops::read_plan(paths)?.ok_or_else(missing)?)
        .map_err(|_| missing())?;
    let journal = ops::read_journal(paths)?.ok_or_else(missing)?;
    let state = ops::read_state(paths, fleet)?;
    verify_journal::<io::Error>(&journal, &plan, fleet, &state).map_err(|_| missing())?;
    if plan.environment != environment
        || plan.fleet != fleet
        || !matches!(
            plan.scope,
            FleetEnsurePlanScope::Full | FleetEnsurePlanScope::InfrastructureBootstrap
        )
        || journal.completion != FleetEnsureCompletion::Converged
        || !state.pending_principals.is_empty()
        || journal.effects.len() != continuation::actions(&plan, &journal).len()
        || journal
            .effects
            .iter()
            .any(|effect| effect.state != EffectState::Applied)
        || state.active_registry.is_none()
    {
        return Err(missing());
    }
    if plan.scope == FleetEnsurePlanScope::InfrastructureBootstrap
        && ops::infrastructure_bootstrap::terminal::read_receipt(paths, &plan, &journal, &state)
            .map_err(|_| missing())?
            .is_none()
    {
        return Err(missing());
    }
    if plan
        .infrastructure_bootstrap
        .as_ref()
        .is_some_and(|source| source.estate_seed.is_some())
        && !ops::infrastructure_bootstrap::publication::read(paths, &plan.plan_sha256)
            .map_err(|_| missing())?
            .is_some_and(|record| record.completed)
    {
        return Err(missing());
    }
    Ok((plan, state))
}
