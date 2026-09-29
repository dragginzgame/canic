//! Drive clean reset through reviewed current infrastructure, imports and workload convergence.
//!
//! Each apply digest belongs to one durable owner. Completed predecessor records are opaque history.

use crate::{
    fleet_ensure::{
        model::{
            DesiredFleet, FleetEnsureCompletion, FleetEnsurePlan, FleetEnsurePlanScope,
            FleetEnsureReport, capacity_import::CapacityImportJournalRecord,
            clean_reinstall::CleanReinstallRecord,
        },
        ops::{
            self, EnsurePaths, EnsurePlatform, EnsureStateError,
            capacity_import::{
                admission::survey::CapacityImportSurveyStore,
                journal::{CapacityImportJournalError, CapacityImportJournalStore},
                publication,
            },
            clean_reinstall as storage,
            infrastructure_bootstrap::{InfrastructureBootstrapError, automatic},
        },
        workflow::{
            self, EnsureWorkflowError, capacity_import::review as imports, infrastructure_bootstrap,
        },
    },
    icp::IcpCli,
};
use candid::Principal;
use canic_core::cdk::utils::hash::decode_hex;
use std::path::Path;

pub use crate::fleet_ensure::view::clean_reinstall::CleanReinstallReport;

/// Cancel only the exact unpaid review; no canister, Ledger or artifact effect occurs.
pub fn cancel_review(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    digest: &str,
) -> Result<
    crate::fleet_ensure::model::clean_reinstall::CleanReinstallCancellationRecord,
    EnsureStateError,
> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)
        .map_err(|_| EnsureStateError::ResetReviewConflict)?;
    storage::cancellation::cancel(
        &EnsurePaths::under(workspace, environment, fleet),
        environment,
        fleet,
        digest,
    )
}

/// Select the normal completed-Fleet reset before attempting to decode predecessor desired state.
pub fn selected(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    reinstall: bool,
    applying: bool,
) -> Result<bool, EnsureStateError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)
        .map_err(|_| EnsureStateError::InvalidTerminalSource)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    storage::cancellation::require_no_pending(&paths)?;
    if reinstall && storage::cancellation::ready_for_review(&paths)? {
        return Ok(true);
    }
    if ops::operation_selection::retirement::pending(&paths)?.is_some() {
        return Ok(reinstall);
    }
    if reinstall
        && !paths.plan.exists()
        && !paths.journal.exists()
        && workspace
            .join(".canic/fleet-ensure/history")
            .join(environment)
            .join(fleet)
            .join("retirements")
            .exists()
    {
        return Ok(true);
    }
    if ops::operation_selection::completed_fleet(&paths, environment, fleet)? {
        return Ok(
            reinstall || (applying && paths.plan.with_file_name("clean-reinstall.json").exists())
        );
    }
    Ok(paths.plan.with_file_name("clean-reinstall.json").exists())
}

/// A same-operation retry uses its frozen current desired authority.
pub fn retained_desired(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    reinstall: bool,
) -> Result<Option<DesiredFleet>, EnsureStateError> {
    let paths = EnsurePaths::under(workspace, environment, fleet);
    if ops::operation_selection::retirement::pending(&paths)?.is_some() {
        return Ok(None);
    }
    if reinstall && ops::operation_selection::completed_fleet(&paths, environment, fleet)? {
        return Ok(None);
    }
    Ok(storage::read(&paths)?.map(|record| record.desired.desired().clone()))
}

/// Review the next bounded phase; every new completed-estate reset starts with current custody.
pub fn review<P: EnsurePlatform>(
    workspace: &Path,
    desired: &DesiredFleet,
    policy: &Path,
    seed: &Path,
    time: u64,
    platform: &mut P,
    icp: &IcpCli,
) -> Result<CleanReinstallReport, EnsureWorkflowError<P::Error>> {
    let paths = EnsurePaths::under(workspace, &desired.environment, &desired.fleet);
    ops::operation_selection::retirement::prepare_bootstrap(&paths, desired)?;
    let record = match storage::read(&paths)? {
        Some(record) if record.desired.desired() == desired => record,
        Some(_) => return Err(EnsureWorkflowError::PlanIntegrity),
        None => storage::bind(&paths, desired, policy, seed)?,
    };
    let Some(plan) = ops::read_plan(&paths)? else {
        let plan = infrastructure_bootstrap::review_clean(
            workspace,
            desired,
            &record.seed,
            time,
            platform,
            icp,
        )?;
        return Ok(CleanReinstallReport::Infrastructure(Box::new(report(
            plan, false,
        ))));
    };
    let journal = ops::read_journal(&paths)?;
    if plan.scope != FleetEnsurePlanScope::InfrastructureBootstrap {
        if journal
            .as_ref()
            .is_some_and(|journal| journal.completion == FleetEnsureCompletion::ReplanRequired)
        {
            return Ok(CleanReinstallReport::Fleet(Box::new(workflow::plan(
                workspace,
                desired,
                &plan.desired_sha256,
                &desired.fleet,
                time,
                platform,
            )?)));
        }
        return Ok(CleanReinstallReport::Fleet(Box::new(report(
            plan,
            journal.is_some_and(|journal| journal.completion == FleetEnsureCompletion::Converged),
        ))));
    }
    if !journal.is_some_and(|journal| journal.completion == FleetEnsureCompletion::Converged) {
        return Ok(CleanReinstallReport::Infrastructure(Box::new(report(
            plan, false,
        ))));
    }
    let completed = storage::completed_roots(&paths)?;
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    for root in &bootstrap.roots {
        let id = desired
            .canisters
            .iter()
            .find(|entry| entry.name == root.root)
            .and_then(|entry| entry.principal.as_deref())
            .and_then(|id| Principal::from_text(id).ok())
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        if completed.contains(&id) {
            continue;
        }
        return review_import(&paths, &record, id, icp)
            .map(|record| CleanReinstallReport::Import(Box::new(record)));
    }
    let digest = storage::convergence_digest(desired, &plan)?;
    Ok(CleanReinstallReport::Fleet(Box::new(workflow::plan(
        workspace,
        desired,
        &digest,
        &desired.fleet,
        time,
        platform,
    )?)))
}

fn review_import<E: std::error::Error + 'static>(
    paths: &EnsurePaths,
    record: &CleanReinstallRecord,
    root: Principal,
    icp: &IcpCli,
) -> Result<CapacityImportJournalRecord, EnsureWorkflowError<E>> {
    let owner = import(CapacityImportJournalStore::open(paths))?;
    if let Some(existing) = import(owner.read())? {
        if existing.plan.authority.root == root && !publication::completed(&existing) {
            return Ok(existing);
        }
        if existing.approved && !publication::completed(&existing) {
            return Err(InfrastructureBootstrapError::Integrity.into());
        }
    }
    let desired = record.desired.desired();
    let ids = desired
        .canisters
        .iter()
        .filter(|entry| entry.kind == crate::fleet_ensure::model::DesiredCanisterKind::Pool)
        .filter(|entry| {
            entry
                .parent
                .as_ref()
                .and_then(|parent| desired.canisters.iter().find(|entry| &entry.name == parent))
                .and_then(|entry| entry.principal.as_deref())
                == Some(root.to_text().as_str())
        })
        .map(|entry| {
            entry
                .principal
                .as_deref()
                .and_then(|id| Principal::from_text(id).ok())
                .ok_or(EnsureWorkflowError::PlanIntegrity)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let digest = storage::import_survey_digest(record, root)?;
    let mut survey = import(CapacityImportSurveyStore::open(&owner, paths, digest, &ids))?;
    let agent = automatic::agent(icp, desired)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(InfrastructureBootstrapError::Io)?;
    let mut samples = Vec::new();
    for id in ids {
        let sample = if let Some(sample) = survey.sample(id) {
            sample.clone()
        } else {
            import(survey.reserve(id))?;
            let sample = import(runtime.block_on(automatic::observe_child(&agent, root, id)))?;
            import(survey.retain(sample.clone()))?;
            sample
        };
        samples.push(sample);
    }
    let request = storage::import_request(paths, record, root, &samples, &agent.read_root_key())?;
    drop(survey);
    drop(owner);
    import(imports::plan(&paths.workspace, &request, icp))
}

/// Apply only the exact retained phase. Receipt replay returns through its original owner.
pub fn apply<P: EnsurePlatform>(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    digest: &str,
    platform: &mut P,
    icp: &IcpCli,
) -> Result<CleanReinstallReport, EnsureWorkflowError<P::Error>> {
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let record = storage::read(&paths)?.ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let import_digest: [u8; 32] = decode_hex(digest)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let owner = import(CapacityImportJournalStore::open(&paths))?;
    let matching = import(owner.read())?.is_some_and(|record| {
        record
            .operation
            .as_ref()
            .is_some_and(|operation| operation.review.review_sha256 == import_digest)
    }) || import(owner.completed_review(import_digest))?.is_some();
    drop(owner);
    if matching {
        return import(imports::apply(
            workspace,
            environment,
            fleet,
            import_digest,
            icp,
        ))
        .map(|record| CleanReinstallReport::Import(Box::new(record)));
    }
    if let Ok(plan) = infrastructure_bootstrap::retained(workspace, environment, fleet, digest) {
        let published = infrastructure_bootstrap::apply_reporting(
            workspace,
            environment,
            fleet,
            digest,
            platform,
        )?;
        return Ok(CleanReinstallReport::Infrastructure(Box::new(
            published
                .execution
                .unwrap_or_else(|| report(plan, published.publication.completed)),
        )));
    }
    let plan = ops::read_plan(&paths)?.ok_or(EnsureWorkflowError::PlanMissing)?;
    Ok(CleanReinstallReport::Fleet(Box::new(workflow::apply(
        workspace,
        record.desired.desired(),
        &plan.desired_sha256,
        fleet,
        digest,
        platform,
    )?)))
}

fn import<T, E: std::error::Error + 'static>(
    value: Result<T, CapacityImportJournalError>,
) -> Result<T, EnsureWorkflowError<E>> {
    value.map_err(|error| InfrastructureBootstrapError::Inspection(error).into())
}

const fn report(plan: FleetEnsurePlan, terminal: bool) -> FleetEnsureReport {
    FleetEnsureReport {
        plan,
        terminal,
        effects_applied: 0,
        funding_review: None,
        actual_conservation: None,
    }
}
