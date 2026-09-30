//! Complete approved capacity handoffs through Root and recover paired inventory publication.
//!
//! Ops owns every record mutation and IC call. This owner retains budgets before effects.

mod handoff;
pub mod review;

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        model::capacity_import::CapacityImportJournalRecord,
        ops::{
            EnsurePaths,
            capacity_import::{
                journal::{CapacityImportJournalError, CapacityImportJournalStore},
                publication,
                transport::CapacityImportTransport,
            },
        },
    },
    icp::IcpCli,
};
use canic_core::dto::pool_import::{PoolImportPhase, PoolImportSourceProgress, PoolImportStatus};

pub use handoff::apply;

/// Finish already confirmed host handoffs using exact retained review authority.
///
/// The caller retains the shared Fleet lock throughout. Terminal replay returns before
/// resolving ICP identity, reading generator inputs or observing subsequently assigned assets.
pub async fn complete(
    store: &CapacityImportJournalStore,
    paths: &EnsurePaths,
    review_sha256: [u8; 32],
    icp: &IcpCli,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    if let Some(completed) = store.completed_review(review_sha256)? {
        return Ok(completed);
    }
    let mut journal = store.read()?.ok_or(CapacityImportJournalError::Integrity)?;
    let operation = journal
        .operation
        .as_ref()
        .ok_or(CapacityImportJournalError::Integrity)?;
    if operation.review.review_sha256 != review_sha256 {
        return Err(CapacityImportJournalError::PublicationConflict);
    }
    if publication::completed(&journal) {
        return Ok(journal);
    }
    if !journal.approved
        || journal.reservation.is_none()
        || !crate::fleet_ensure::ops::capacity_import::journal::all_custody_ready(&journal)
    {
        return Err(CapacityImportJournalError::Unresolved);
    }
    if !operation.publication_complete {
        publication::verify_inputs(paths, &journal, operation.publication_started)?;
    }
    let transport = CapacityImportTransport::from_icp(icp)?;
    if operation.settled_status_candid_hex.is_none() {
        transport.verify_destination(&journal.plan).await?;
        let observed = finish_root(store, &mut journal, &transport).await?;
        journal = publication::retain_settled(&journal, &observed)?;
        store.save(&journal)?;
    }
    journal = publication::publish(store, paths)?;
    let mut observed = transport.root_status(&journal.plan).await?;
    if !matches!(observed.phase, PoolImportPhase::Released { .. }) {
        let prepared = transport.prepare_release_root(&journal).await?;
        journal = publication::reserve_submission(&journal, "release")?;
        store.save(&journal)?;
        // If this reply is lost, reopening queries the retained Released receipt first.
        observed = prepared.submit().await?;
    }
    journal = publication::retain_released(&journal, &observed)?;
    store.save(&journal)?;
    Ok(journal)
}

async fn finish_root(
    store: &CapacityImportJournalStore,
    journal: &mut CapacityImportJournalRecord,
    transport: &CapacityImportTransport,
) -> Result<PoolImportStatus, CapacityImportJournalError> {
    let mut observed = transport.root_status(&journal.plan).await?;
    for index in 0..journal.plan.sources.len() {
        while let Some(step) = progress_step(
            &observed.progress[index],
            journal.plan.sources[index].binding.stopped,
        ) {
            let prepared = transport
                .prepare_advance_root(journal, journal.plan.sources[index].binding.canister_id)
                .await?;
            *journal = publication::reserve_submission(journal, &format!("{index}:{step}"))?;
            store.save(journal)?;
            observed = prepared.submit().await?;
        }
    }
    if observed.root_receipt.is_none() {
        let prepared = transport.prepare_settle_root(journal).await?;
        *journal = publication::reserve_submission(journal, "settle")?;
        store.save(journal)?;
        observed = prepared.submit().await?;
    }
    Ok(observed)
}

const fn progress_step(
    progress: &PoolImportSourceProgress,
    originally_stopped: bool,
) -> Option<&'static str> {
    match progress {
        PoolImportSourceProgress::AwaitingHandoff if !originally_stopped => Some("stop"),
        PoolImportSourceProgress::AwaitingHandoff | PoolImportSourceProgress::Stopped => {
            Some("controllers")
        }
        PoolImportSourceProgress::ControllersIssued => Some("confirm"),
        PoolImportSourceProgress::ControllersConfirmed => Some("uninstall"),
        PoolImportSourceProgress::StopIssued => Some("confirm_stop"),
        PoolImportSourceProgress::UninstallIssued => Some("cleared"),
        PoolImportSourceProgress::Ready(_) => None,
    }
}
