//! Module: fleet_ensure::ops::independent_effects
//!
//! Responsibility: retain evidence and submit bounded independent Store or pool effects.
//! Does not own: prerequisite admission, journal publication, retries or completion decisions.
//! Boundary: drain every submitted call and retain results in plan order.

use crate::{
    fleet_ensure::{
        model::{
            CurrentFleetProtocolAction, EffectRecord, EffectState, EnsureAction,
            MAX_INDEPENDENT_EFFECTS_IN_FLIGHT,
        },
        ops::{
            EffectObservation, EffectOutcome, action_sha256, current_protocol::CurrentProtocolError,
        },
    },
    icp::IcpCli,
};
use std::{panic::resume_unwind, path::Path, thread};

/// One exact independent effect and its persisted intent, supplied by the Fleet workflow.
pub struct IndependentEffect<'a> {
    pub action: &'a EnsureAction,
    pub record: &'a EffectRecord,
}

pub(super) fn apply(
    icp: &IcpCli,
    root: &Path,
    uploads: &[IndependentEffect<'_>],
) -> Result<Vec<Result<EffectOutcome, CurrentProtocolError>>, CurrentProtocolError> {
    if uploads.len() > MAX_INDEPENDENT_EFFECTS_IN_FLIGHT
        || uploads.iter().any(|upload| !upload.is_exact())
    {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    // Keep a result for every input rather than short-circuiting on the first error.
    Ok(drain(uploads, |upload| {
        crate::fleet_ensure::ops::current_protocol::apply(icp, root, upload.action)
    }))
}

impl IndependentEffect<'_> {
    fn is_exact(&self) -> bool {
        let independent = matches!(self.action,
            EnsureAction::FleetProtocol { action, .. }
            if matches!(action.as_ref(), CurrentFleetProtocolAction::PublishStoreChunk { request }
                if request.preparation.is_none() && request.chunk_index > 0)
                || matches!(action.as_ref(), CurrentFleetProtocolAction::ReconcilePoolAsset { .. }));
        independent
            && self.record.state == EffectState::Intent
            && self.record.action_sha256 == action_sha256(self.action)
    }
}

fn drain<T: Sync, U: Send>(inputs: &[T], submit: impl Fn(&T) -> U + Sync) -> Vec<U> {
    assert!(inputs.len() <= MAX_INDEPENDENT_EFFECTS_IN_FLIGHT);
    let results = thread::scope(|scope| {
        let workers = inputs
            .iter()
            .map(|input| scope.spawn(|| submit(input)))
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .map(thread::ScopedJoinHandle::join)
            .collect::<Vec<_>>()
    });
    results
        .into_iter()
        .map(|result| result.unwrap_or_else(|panic| resume_unwind(panic)))
        .collect()
}

pub(in crate::fleet_ensure) fn retain_outcome(
    record: &mut EffectRecord,
    outcome: EffectOutcome,
) -> bool {
    if outcome.created_principal.is_some() || outcome.post_cycles.is_some() {
        return false;
    }
    record.receipt = outcome.receipt;
    record.state = EffectState::Issued;
    true
}

pub(in crate::fleet_ensure) fn retain_observation(
    record: &mut EffectRecord,
    observed: EffectObservation,
    source: Option<u128>,
    destination: Option<u128>,
) {
    record.post_cycles = source.or(record.post_cycles);
    record.destination_post_cycles = destination.or(record.destination_post_cycles);
    record.progress_identity = Some(observed.progress_identity);
    if observed.applied {
        record.state = EffectState::Applied;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Barrier,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn overlaps_bounded_uploads_and_retains_successes_after_failure() {
        let barrier = Barrier::new(MAX_INDEPENDENT_EFFECTS_IN_FLIGHT);
        let completed = AtomicUsize::new(0);
        let results = drain(&[0, 1, 2, 3], |index| {
            barrier.wait();
            completed.fetch_add(1, Ordering::SeqCst);
            if *index < 2 { Err(*index) } else { Ok(*index) }
        });
        assert_eq!(results, [Err(0), Err(1), Ok(2), Ok(3)]);
        assert_eq!(
            completed.load(Ordering::SeqCst),
            MAX_INDEPENDENT_EFFECTS_IN_FLIGHT
        );
    }
}
