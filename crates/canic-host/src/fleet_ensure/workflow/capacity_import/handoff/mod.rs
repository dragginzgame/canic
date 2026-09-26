//! Sequence exact signed host handoffs before the Root-owned reset and publication workflow.

use crate::{
    fleet_ensure::{
        model::{EffectState, capacity_import::CapacityImportJournalRecord},
        ops::{
            EnsurePaths,
            capacity_import::{
                journal::{self, CapacityImportJournalError, CapacityImportJournalStore},
                observation::CapacityImportObserver,
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
    if record.handoffs.iter().all(|handoff| {
        handoff
            .effect
            .as_ref()
            .is_some_and(|effect| effect.state == EffectState::Applied)
    }) {
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
    *record = publication::reserve_inspection(record, record.plan.authority.root)?;
    store.save(record)?;
    let destination = observer.destination(&record.plan).await?;
    let mut sources = Vec::with_capacity(record.plan.sources.len());
    for index in 0..record.plan.sources.len() {
        let canister = record.plan.sources[index].binding.canister_id;
        *record = publication::reserve_inspection(record, canister)?;
        store.save(record)?;
        sources.push(observer.source(&record.plan, canister).await?);
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
        *record = publication::reserve_submission(record, "reserve")?;
        store.save(record)?;
        transport.reserve_root(record).await?
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
    if record.handoffs[index]
        .effect
        .as_ref()
        .is_some_and(|effect| effect.state == EffectState::Applied)
    {
        return Ok(());
    }
    if record.handoffs[index].effect.is_none() {
        *record = publication::reserve_inspection(record, id)?;
        store.save(record)?;
        let source = observer.source(&record.plan, id).await?;
        let request = transport.prepare(&record.plan, id)?;
        *record = journal::prepare_handoff(record, &source, request)?;
        store.save(record)?;
    }
    loop {
        if journal::rejection::pending(&record.handoffs[index]) {
            publication::require_submission_allowance(record, &format!("{index}:handoff"))?;
            *record = publication::reserve_inspection(record, id)?;
            store.save(record)?;
            let source = observer.source(&record.plan, id).await?;
            let request = transport.prepare(&record.plan, id)?;
            *record = journal::rejection::renew(record, &source, request)?;
            store.save(record)?;
        }
        match submit_or_reconcile(store, record, transport, index).await? {
            HandoffOutcome::Rejected(rejected) => {
                *record = journal::rejection::retain(record, id, &rejected)?;
                store.save(record)?;
            }
            HandoffOutcome::Completed(completion) => {
                *record = publication::reserve_inspection(record, id)?;
                store.save(record)?;
                let sample = observer.source(&record.plan, id).await?;
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
    *record = publication::reserve_submission(record, &format!("{index}:handoff"))?;
    store.save(record)?;
    if record.handoffs[index]
        .effect
        .as_ref()
        .is_some_and(|effect| effect.state == EffectState::Intent)
    {
        *record = journal::issue_handoff(record, id)?;
        store.save(record)?;
    }
    match transport.submit(record, id).await {
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
