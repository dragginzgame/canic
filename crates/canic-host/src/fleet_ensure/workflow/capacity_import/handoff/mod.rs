//! Sequence exact signed host handoffs before the Root-owned reset and publication workflow.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        model::{EffectState, capacity_import::CapacityImportJournalRecord},
        ops::{
            EnsurePaths,
            capacity_import::{
                journal::{self, CapacityImportJournalError, CapacityImportJournalStore},
                observation::{CapacityImportObserver, PreparedCapacityImportObservation},
                publication, reservation_evidence,
                transport::{CapacityImportTransport, HandoffOutcome},
            },
        },
        workflow::capacity_import::complete,
    },
    icp::IcpCli,
};
use canic_core::dto::pool_import::PoolImportIdentity;
use std::time::Duration;

/// Apply one exact operator review under the held Fleet lock, preserving every issued request.
///
/// Completed replay precedes observation and ICP identity resolution. Unknown outcomes retain
/// their original signed bytes; neither restart nor an observation failure creates new ingress.
pub async fn apply(
    store: &CapacityImportJournalStore,
    paths: &EnsurePaths,
    review_sha256: [u8; 32],
    icp: &IcpCli,
    observer: &mut impl CapacityImportObserver,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    if let Some(completed) = store.completed_review(review_sha256)? {
        return Ok(completed);
    }
    let mut record = store.read()?.ok_or(CapacityImportJournalError::Integrity)?;
    let operation = record
        .operation
        .as_ref()
        .ok_or(CapacityImportJournalError::Integrity)?;
    if operation.review.review_sha256 != review_sha256 {
        return Err(CapacityImportJournalError::PublicationConflict);
    }
    if record.approved && record.reservation.is_some() && journal::all_custody_ready(&record) {
        return complete(store, paths, review_sha256, icp).await;
    }
    publication::verify_inputs(paths, &record, false)?;
    let transport = CapacityImportTransport::from_icp(icp)?;
    transport.verify_destination(&record.plan).await?;
    if !record.approved {
        approve(store, &mut record, observer).await?;
    }
    reserve(store, &mut record, &transport).await?;
    for index in 0..record.handoffs.len() {
        publication::verify_inputs(paths, &record, false)?;
        handoff(store, &mut record, &transport, observer, index).await?;
    }
    complete(store, paths, review_sha256, icp).await
}

async fn approve(
    store: &CapacityImportJournalStore,
    record: &mut CapacityImportJournalRecord,
    observer: &mut impl CapacityImportObserver,
) -> Result<(), CapacityImportJournalError> {
    let destination = observer.prepare_destination(&record.plan).await?;
    let mut prepared_sources = Vec::with_capacity(record.plan.sources.len());
    for source in &record.plan.sources {
        let canister = source.binding.canister_id;
        prepared_sources.push((
            canister,
            observer.prepare_source(&record.plan, canister).await?,
        ));
    }
    *record = publication::reserve_inspection(record, record.plan.authority.root)?;
    store.save(record)?;
    let destination = destination.observe().await?;
    let mut sources = Vec::with_capacity(record.plan.sources.len());
    for (canister, prepared) in prepared_sources {
        *record = publication::reserve_inspection(record, canister)?;
        store.save(record)?;
        sources.push(prepared.observe().await?);
    }
    *record = journal::approve(record, record.plan.plan_sha256, &destination, &sources)?;
    store.save(record)?;
    Ok(())
}

async fn reserve(
    store: &CapacityImportJournalStore,
    record: &mut CapacityImportJournalRecord,
    transport: &CapacityImportTransport,
) -> Result<(), CapacityImportJournalError> {
    let context = transport.verify_destination(&record.plan).await?;
    let expected = PoolImportIdentity {
        sequence: record.plan.authority.import_sequence,
        plan_sha256: record.plan.plan_sha256,
    };
    let status = if context.active_import == Some(expected) {
        transport.root_status(&record.plan).await?
    } else {
        if context.active_import.is_some() || record.reservation.is_some() {
            return Err(CapacityImportJournalError::Conflict);
        }
        let prepared = transport.prepare_reserve_root(record).await?;
        *record = publication::reserve_submission(record, "reserve")?;
        store.save(record)?;
        prepared.submit().await?
    };
    *record = journal::reserve(record, reservation_evidence(&record.plan, &status)?)?;
    store.save(record)?;
    Ok(())
}

async fn handoff(
    store: &CapacityImportJournalStore,
    record: &mut CapacityImportJournalRecord,
    transport: &CapacityImportTransport,
    observer: &mut impl CapacityImportObserver,
    index: usize,
) -> Result<(), CapacityImportJournalError> {
    let id = record.handoffs[index].canister_id;
    if journal::custody_ready(record, id) {
        return Ok(());
    }
    if record.handoffs[index].effect.is_none() {
        let prepared = observer.prepare_source(&record.plan, id).await?;
        let request = transport.prepare(&record.plan, id)?;
        *record = publication::reserve_inspection(record, id)?;
        store.save(record)?;
        let source = prepared.observe().await?;
        *record = journal::prepare_handoff(record, &source, request)?;
        store.save(record)?;
    }
    loop {
        if journal::retirement::pending(&record.handoffs[index]) {
            let prepared = observer.prepare_source(&record.plan, id).await?;
            let request = transport.prepare(&record.plan, id)?;
            *record = publication::reserve_inspection(record, id)?;
            store.save(record)?;
            let source = prepared.observe().await?;
            *record = journal::retirement::reconcile(record, &source, request)?;
            store.save(record)?;
            if journal::custody_ready(record, id) {
                return Ok(());
            }
        }
        match submit_or_reconcile(store, record, transport, index).await? {
            HandoffOutcome::Retired(rejected) => {
                *record = journal::retirement::retain(record, id, &rejected)?;
                store.save(record)?;
            }
            HandoffOutcome::Completed(completion) => {
                let prepared = observer.prepare_source(&record.plan, id).await?;
                *record = publication::reserve_inspection(record, id)?;
                store.save(record)?;
                let sample = prepared.observe().await?;
                *record = journal::observe_handoff(record, &sample, &completion)?;
                store.save(record)?;
                return Ok(());
            }
        }
    }
}

async fn submit_or_reconcile(
    store: &CapacityImportJournalStore,
    record: &mut CapacityImportJournalRecord,
    transport: &CapacityImportTransport,
    index: usize,
) -> Result<HandoffOutcome, CapacityImportJournalError> {
    let id = record.handoffs[index].canister_id;
    if record.handoffs[index]
        .effect
        .as_ref()
        .is_some_and(|effect| effect.state == EffectState::Issued)
    {
        // A retained reply remains useful after ingress expiry; reconcile before resubmission.
        match transport.completion(record, id).await {
            Ok(completion) => return Ok(completion),
            Err(CapacityImportJournalError::Unresolved) => {}
            Err(error) => return Err(error),
        }
    }
    let unissued = record.handoffs[index]
        .effect
        .as_ref()
        .is_some_and(|effect| effect.state == EffectState::Intent);
    let refreshed = if unissued {
        let request = transport.prepare(&record.plan, id)?;
        journal::retirement::refresh_unissued(record, id, request)?
    } else {
        record.clone()
    };
    let prepared = transport.prepare_submission(&refreshed, id).await?;
    let mut issued = publication::reserve_submission(&refreshed, &format!("{index}:handoff"))?;
    if unissued {
        issued = journal::issue_handoff(&issued, id)?;
    }
    // Charge the attempt and retain Issued together, before submitting any bytes.
    store.save(&issued)?;
    *record = issued;
    match prepared.submit(record).await {
        Ok(()) | Err(CapacityImportJournalError::Unresolved) => {}
        Err(error) => return Err(error),
    }
    await_completion(transport, record, id).await
}

async fn await_completion(
    transport: &CapacityImportTransport,
    record: &CapacityImportJournalRecord,
    canister: candid::Principal,
) -> Result<HandoffOutcome, CapacityImportJournalError> {
    tokio::time::timeout(Duration::from_secs(12), async {
        for _ in 0..24 {
            match transport.completion(record, canister).await {
                Ok(completion) => return Ok(completion),
                Err(CapacityImportJournalError::Unresolved) => {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
                Err(error) => return Err(error),
            }
        }
        Err(CapacityImportJournalError::Unresolved)
    })
    .await
    .map_err(|_| CapacityImportJournalError::Unresolved)?
}
