//! Review and publish fresh completed-estate reset authority before the ordinary effect driver.
//!
//! Source evidence remains byte-bound and archived; current Ensure owns all installs and payments.

use crate::{
    fleet_ensure::{
        model::{
            ActualCycleConservation, DesiredFleet, FleetEnsureJournalRecord, FleetEnsurePlan,
            FleetObservation,
        },
        ops::{
            self, EnsurePaths, EnsurePlatform, completed_handoff as publication,
            completed_preparation as preparation, completed_reset as reset, retained_contract,
        },
        view::completed_reset::{CompletedResetBalancesView, CompletedResetReviewView},
        workflow::EnsureWorkflowError,
    },
    icp::IcpCli,
};
use std::path::Path;

pub use reset::CompletedResetError;

/// Archive consumed approvals before a new explicit reinstall; resume interrupted local retirement.
/// No canister state, current operation, payment or artifact is changed.
pub fn retire_completed_authority(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    new_reinstall: bool,
) -> Result<(), CompletedResetError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    if !new_reinstall && !publication::retirement::pending(&paths) {
        return Ok(());
    }
    let _lock = ops::lock_completed_source(&paths)?;
    if new_reinstall {
        ops::operation_selection::archive::capture(&paths, environment, fleet)?;
        publication::retirement::retire(&paths)?;
    } else {
        publication::retirement::recover(&paths)?;
    }
    Ok(())
}

/// Read staged or committed current authority without decoding predecessor executable records.
pub fn review(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<Option<CompletedResetReviewView>, CompletedResetError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let committed = publication::committed(&paths)?;
    let is_committed = committed.is_some();
    let selected = match committed {
        Some(review) => Some(review),
        None => publication::review(&paths)?,
    };
    let Some(publication) = selected else {
        return Ok(None);
    };
    let plan = publication::replacement_plan(&paths, &publication)?;
    if plan
        .reinstall
        .as_ref()
        .is_none_or(|intent| intent.completed_reset.is_none())
    {
        return Err(CompletedResetError::Conflict);
    }
    Ok(Some(CompletedResetReviewView {
        publication,
        plan,
        committed: is_committed,
    }))
}

/// Whether explicit source preparation has completed; no artifact build or paid call is issued.
pub fn prepared(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<bool, CompletedResetError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let Some(review) = preparation::review(&paths).map_err(Box::new)? else {
        return Ok(false);
    };
    Ok(preparation::journal(&paths, &review)
        .map_err(Box::new)?
        .is_some_and(|journal| journal.prepared))
}

/// Bind current artifacts, exact physical imports and the complete wipe/funding plan into review.
pub fn plan(
    workspace: &Path,
    desired: &DesiredFleet,
    digest: &str,
    time: u64,
    icp: &IcpCli,
) -> Result<CompletedResetReviewView, CompletedResetError> {
    crate::fleet_ensure::policy::validate_path_labels(&desired.environment, &desired.fleet)?;
    let paths = EnsurePaths::under(workspace, &desired.environment, &desired.fleet);
    let _lock = ops::lock_completed_preparation(&paths)?;
    if let Some(review) = review(workspace, &desired.environment, &desired.fleet)? {
        if review.plan.desired_sha256 != digest {
            return Err(CompletedResetError::Conflict);
        }
        return Ok(review);
    }
    let source = retained_contract::inspect_completed_source(
        workspace,
        &desired.environment,
        &desired.fleet,
    )
    .map_err(Box::new)?;
    let observed = inspect(workspace, &desired.environment, &desired.fleet, icp)?;
    require_seals(&paths, icp)?;
    let target = reset::compile(&paths, desired, digest, time, &source, &observed)?;
    let publication = publication::stage_locked(
        &paths,
        &target.plan,
        &target.journal,
        &target.state,
        observed.custody(),
    )?;
    Ok(CompletedResetReviewView {
        publication,
        plan: target.plan,
        committed: false,
    })
}

/// Commit approved current authority; already committed intent resumes without touching the source.
/// The caller then passes the returned plan to the existing Ensure apply entry point.
pub fn approve(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    approval: &str,
    icp: &IcpCli,
) -> Result<FleetEnsurePlan, CompletedResetError> {
    let review = review(workspace, environment, fleet)?.ok_or(CompletedResetError::Conflict)?;
    if review.publication.review_sha256 != approval {
        return Err(CompletedResetError::Conflict);
    }
    let paths = EnsurePaths::under(workspace, environment, fleet);
    if review.committed {
        publication::adopt(&paths, approval, None)?;
        return Ok(review.plan);
    }
    let source = retained_contract::inspect_completed_source(workspace, environment, fleet)
        .map_err(Box::new)?;
    let observed = inspect(workspace, environment, fleet, icp)?;
    require_seals(&paths, icp)?;
    let desired = review
        .plan
        .reviewed_desired
        .as_ref()
        .ok_or(CompletedResetError::Conflict)?
        .desired();
    let target = reset::compile(
        &paths,
        desired,
        &review.plan.desired_sha256,
        review.plan.planned_at_time,
        &source,
        &observed,
    )?;
    if target.plan != review.plan {
        return Err(CompletedResetError::Conflict);
    }
    publication::adopt(&paths, approval, Some(observed.custody()))?;
    Ok(review.plan)
}

fn require_seals(paths: &EnsurePaths, icp: &IcpCli) -> Result<(), CompletedResetError> {
    let review = preparation::review(paths)
        .map_err(Box::new)?
        .ok_or(CompletedResetError::Conflict)?;
    for action in &review.actions {
        if !preparation::sealed(icp, paths, &review, action).map_err(Box::new)? {
            return Err(CompletedResetError::Conflict);
        }
    }
    Ok(())
}

fn inspect(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    icp: &IcpCli,
) -> Result<crate::fleet_ensure::CompletedEstateMembershipView, CompletedResetError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(retained_contract::inspect_completed_membership(
            workspace,
            environment,
            fleet,
            icp,
        ))
        .map_err(|error| Box::new(error).into())
}

pub(super) fn sample<P: EnsurePlatform>(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    observation: &mut FleetObservation,
    platform: &mut P,
) -> Result<Option<CompletedResetBalancesView>, EnsureWorkflowError<P::Error>> {
    let Some(intent) = plan
        .reinstall
        .as_ref()
        .filter(|intent| intent.completed_reset.is_some())
    else {
        return Ok(None);
    };
    reset::terminal::consume(paths, plan)?;
    let balances = platform
        .completed_reset_balances(intent)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    reset::terminal::attach(observation, &balances)?;
    Ok(Some(balances))
}

pub(super) fn verify<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    actual: &ActualCycleConservation,
    balances: Option<&CompletedResetBalancesView>,
) -> Result<(), EnsureWorkflowError<E>> {
    let source = plan
        .reinstall
        .as_ref()
        .and_then(|intent| intent.completed_reset.as_deref());
    match (source, balances) {
        (None, None) => Ok(()),
        (Some(source), Some(balances)) => {
            if journal.initial_operator_cycles != source.operator_cycles {
                return Err(EnsureWorkflowError::JournalIntegrity);
            }
            let operator_source =
                crate::fleet_ensure::policy::operator_mint::operator_source(journal)
                    .ok_or(EnsureWorkflowError::JournalIntegrity)?;
            let funded = crate::fleet_ensure::workflow::funding_plan::<E>(plan, journal)?;
            if crate::fleet_ensure::policy::reinstall::completed::conserved(
                source,
                balances,
                actual,
                funded.conservation.maximum_execution_burn_cycles,
                operator_source,
            ) {
                Ok(())
            } else {
                Err(EnsureWorkflowError::Conservation(
                    "completed-source native, reserved or Ledger conservation is unproven".into(),
                ))
            }
        }
        _ => Err(EnsureWorkflowError::JournalIntegrity),
    }
}
