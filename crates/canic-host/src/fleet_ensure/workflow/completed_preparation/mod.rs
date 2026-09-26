//! Sequence a reviewed completed-source preparation through the existing seal effect owner.
//!
//! Original Fleet evidence remains unchanged. Preparation does not authorize reinstall or funding.

use crate::{
    fleet_ensure::{
        model::completed_handoff::preparation::{
            CompletedPreparationJournalRecord, CompletedPreparationReviewRecord,
        },
        ops::{self, EnsurePaths, completed_preparation as preparation, retained_contract},
    },
    icp::IcpCli,
};
use std::path::Path;

pub use preparation::CompletedPreparationError;

/// Inspect the current preparation without source decoding, builds or network calls.
pub fn review(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<Option<CompletedPreparationReviewRecord>, CompletedPreparationError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)
        .map_err(|_| CompletedPreparationError::Conflict)?;
    preparation::review(&EnsurePaths::under(workspace, environment, fleet))
}

/// Review source custody/membership and stage the exact authority-seal effects before any build.
pub fn plan(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    icp: &IcpCli,
) -> Result<CompletedPreparationReviewRecord, CompletedPreparationError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)
        .map_err(|_| CompletedPreparationError::Conflict)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let _lock = ops::lock_completed_preparation(&paths)?;
    if let Some(review) = preparation::review(&paths)?
        && preparation::journal(&paths, &review)?.is_some()
    {
        return Ok(review);
    }
    let source = retained_contract::inspect_completed_source(workspace, environment, fleet)
        .map_err(Box::new)?;
    let observed = inspect(workspace, environment, fleet, icp)?;
    let review = preparation::compile(&paths, &source, &observed, environment, fleet)?;
    preparation::stage(&paths, &review)?;
    Ok(review)
}

/// Apply only the exact reviewed preparation; terminal replay performs no network calls.
pub fn apply(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    approval: &str,
    icp: &IcpCli,
) -> Result<CompletedPreparationJournalRecord, CompletedPreparationError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)
        .map_err(|_| CompletedPreparationError::Conflict)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let _lock = ops::lock_completed_preparation(&paths)?;
    let review = preparation::review(&paths)?.ok_or(CompletedPreparationError::Conflict)?;
    if review.review_sha256 != approval
        || review.environment != environment
        || review.fleet != fleet
    {
        return Err(CompletedPreparationError::Conflict);
    }
    let retained = preparation::journal(&paths, &review)?;
    if let Some(journal) = retained.as_ref().filter(|journal| journal.prepared) {
        return Ok(journal.clone());
    }
    let source = retained_contract::inspect_completed_source(workspace, environment, fleet)
        .map_err(Box::new)?;
    let observed = inspect(workspace, environment, fleet, icp)?;
    preparation::revalidate(&paths, &review, &source, &observed)?;
    let mut journal = match retained {
        Some(journal) => journal,
        None => preparation::begin(&paths, &review)?,
    };
    for (index, action) in review.actions.iter().enumerate() {
        if journal.effects[index].before.is_none() {
            preparation::consume(&paths, &mut journal, index, true)?;
            let cycles = preparation::balance(icp, &review, action)?;
            preparation::inventory::require_headroom(&review, action.name(), cycles)?;
            preparation::retain_balance(&paths, &mut journal, index, cycles, false)?;
        }
        let sealed = preparation::sealed(icp, &paths, &review, action)?;
        if journal.effects[index].applied && !sealed {
            return Err(CompletedPreparationError::Conflict);
        }
        if !journal.effects[index].applied {
            if !sealed {
                preparation::consume(&paths, &mut journal, index, false)?;
                preparation::submit(icp, &paths, &review, action)?;
            }
            preparation::mark_applied(&paths, &mut journal, index)?;
        }
        if journal.effects[index].after.is_none() {
            preparation::consume(&paths, &mut journal, index, true)?;
            let cycles = preparation::balance(icp, &review, action)?;
            preparation::retain_balance(&paths, &mut journal, index, cycles, true)?;
        }
    }
    for name in preparation::inventory::pending(&review, &journal) {
        preparation::inventory::consume(&paths, &mut journal, &name)?;
        let balance = preparation::inventory::observe(&paths, icp, &review, &name)?;
        preparation::inventory::retain(&paths, &mut journal, &name, balance)?;
    }
    preparation::finish(&paths, &review, &mut journal)?;
    Ok(journal)
}

fn inspect(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    icp: &IcpCli,
) -> Result<crate::fleet_ensure::CompletedEstateMembershipView, CompletedPreparationError> {
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
