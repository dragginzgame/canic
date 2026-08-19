//! Module: ops::storage::async_job_recovery
//!
//! Responsibility: claim, finish, abandon, and inspect durable async-job attempts.
//! Does not own: timer registration, provider state, retry timing, or domain work.
//! Boundary: workflows provide observed time and lease deadlines; ops commits exact fences.

#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::storage::stable::async_job_recovery::AsyncJobRecoveryData;
use crate::{
    InternalError,
    model::replay::OperationId,
    storage::stable::async_job_recovery::{
        AsyncAttemptFenceRecord, AsyncAttemptLeaseRecord, AsyncJobRecoveryStore,
        ReplaySafeAsyncAttemptFenceRecord, ReplaySafeAsyncAttemptLeaseRecord,
    },
};
use sha2::{Digest, Sha256};

const ASYNC_JOB_RECOVERY_OPERATION_ID_DOMAIN: &[u8] = b"canic-async-job-recovery-operation:v1";

/// Closed identities for recovery-critical asynchronous domain jobs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AsyncJobOwner {
    AuthRenewal,
    CanisterPoolMaintenance,
    CycleTopup,
    PlacementReceiptAcknowledgement,
}

/// Exact durable attempt token. Only this token may finish its active lease.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsyncJobAttempt {
    owner: AsyncJobOwner,
    attempt_generation: u64,
    operation_generation: Option<u64>,
    lease_expires_at_ns: u64,
}

impl AsyncJobAttempt {
    /// Return the deterministic cycle-funding operation identity, when owned.
    #[must_use]
    pub fn operation_id(self) -> Option<OperationId> {
        self.operation_generation.map(|operation_generation| {
            let mut hasher = Sha256::new();
            hasher.update(ASYNC_JOB_RECOVERY_OPERATION_ID_DOMAIN);
            hasher.update(operation_generation.to_be_bytes());
            OperationId::from_bytes(hasher.finalize().into())
        })
    }

    /// Return the authoritative business-attempt lease deadline.
    #[must_use]
    pub const fn lease_expires_at_ns(self) -> u64 {
        self.lease_expires_at_ns
    }
}

/// Result of attempting to enter one serial async-job owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AsyncJobClaim {
    Acquired(AsyncJobAttempt),
    Busy { retry_at_ns: u64 },
}

/// Domain completion relevant to exact cycle-funding retry identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AsyncJobCompletion {
    Success,
    RetryableFailure,
    InvariantFailure,
}

/// Deterministic stable-state operations for serial async-job attempts.
pub struct AsyncJobRecoveryOps;

impl AsyncJobRecoveryOps {
    /// Claim a fresh attempt, take over an expired attempt, or coalesce behind its lease.
    pub fn claim(
        owner: AsyncJobOwner,
        now_ns: u64,
        lease_expires_at_ns: u64,
    ) -> Result<AsyncJobClaim, InternalError> {
        if lease_expires_at_ns <= now_ns {
            return Err(InternalError::invariant());
        }
        let mut state = AsyncJobRecoveryStore::get();
        let claim = match owner {
            AsyncJobOwner::AuthRenewal => {
                claim_attempt(&mut state.auth_renewal, owner, now_ns, lease_expires_at_ns)
            }
            AsyncJobOwner::CanisterPoolMaintenance => claim_attempt(
                &mut state.canister_pool_maintenance,
                owner,
                now_ns,
                lease_expires_at_ns,
            ),
            AsyncJobOwner::CycleTopup => claim_replay_safe_attempt(
                &mut state.cycle_topup,
                owner,
                now_ns,
                lease_expires_at_ns,
            ),
            AsyncJobOwner::PlacementReceiptAcknowledgement => claim_attempt(
                &mut state.placement_receipt_acknowledgement,
                owner,
                now_ns,
                lease_expires_at_ns,
            ),
        }?;
        if matches!(claim, AsyncJobClaim::Acquired(_)) {
            AsyncJobRecoveryStore::replace(state);
        }
        Ok(claim)
    }

    /// Finish only the exact active attempt and preserve only cycle-funding retry identity.
    pub fn finish(
        attempt: AsyncJobAttempt,
        completion: AsyncJobCompletion,
    ) -> Result<bool, InternalError> {
        let mut state = AsyncJobRecoveryStore::get();
        let exact = match attempt.owner {
            AsyncJobOwner::AuthRenewal => finish_attempt(&mut state.auth_renewal, attempt),
            AsyncJobOwner::CanisterPoolMaintenance => {
                finish_attempt(&mut state.canister_pool_maintenance, attempt)
            }
            AsyncJobOwner::CycleTopup => {
                finish_replay_safe_attempt(&mut state.cycle_topup, attempt, completion)
            }
            AsyncJobOwner::PlacementReceiptAcknowledgement => {
                finish_attempt(&mut state.placement_receipt_acknowledgement, attempt)
            }
        };
        if exact {
            AsyncJobRecoveryStore::replace(state);
        }
        Ok(exact)
    }

    /// Return the active durable lease deadline for snapshot admission checks.
    #[must_use]
    pub fn active_lease_deadline(owner: AsyncJobOwner) -> Option<u64> {
        let state = AsyncJobRecoveryStore::get();
        match owner {
            AsyncJobOwner::AuthRenewal => lease_deadline(&state.auth_renewal),
            AsyncJobOwner::CanisterPoolMaintenance => {
                lease_deadline(&state.canister_pool_maintenance)
            }
            AsyncJobOwner::CycleTopup => state
                .cycle_topup
                .active
                .as_ref()
                .map(|active| active.lease_expires_at_ns),
            AsyncJobOwner::PlacementReceiptAcknowledgement => {
                lease_deadline(&state.placement_receipt_acknowledgement)
            }
        }
    }

    /// Return the expired active-lease deadline that requires one serial takeover.
    #[must_use]
    pub fn expired_deadline(owner: AsyncJobOwner, now_ns: u64) -> Option<u64> {
        Self::active_lease_deadline(owner).filter(|deadline_ns| *deadline_ns <= now_ns)
    }

    /// Clear active work when the domain owner is deliberately stopped.
    pub fn abandon(owner: AsyncJobOwner) {
        let mut state = AsyncJobRecoveryStore::get();
        match owner {
            AsyncJobOwner::AuthRenewal => state.auth_renewal.active = None,
            AsyncJobOwner::CanisterPoolMaintenance => {
                state.canister_pool_maintenance.active = None;
            }
            AsyncJobOwner::CycleTopup => {
                state.cycle_topup.active = None;
                state.cycle_topup.pending_operation_generation = None;
            }
            AsyncJobOwner::PlacementReceiptAcknowledgement => {
                state.placement_receipt_acknowledgement.active = None;
            }
        }
        AsyncJobRecoveryStore::replace(state);
    }

    #[cfg(test)]
    pub(crate) fn reset_for_tests() {
        AsyncJobRecoveryStore::import(AsyncJobRecoveryData::default());
    }
}

fn claim_attempt(
    current: &mut AsyncAttemptFenceRecord,
    owner: AsyncJobOwner,
    now_ns: u64,
    lease_expires_at_ns: u64,
) -> Result<AsyncJobClaim, InternalError> {
    if let Some(active) = &current.active
        && active.lease_expires_at_ns > now_ns
    {
        return Ok(AsyncJobClaim::Busy {
            retry_at_ns: active.lease_expires_at_ns,
        });
    }
    let attempt_generation = current
        .last_attempt_generation
        .checked_add(1)
        .ok_or_else(InternalError::invariant)?;
    current.last_attempt_generation = attempt_generation;
    current.active = Some(AsyncAttemptLeaseRecord {
        attempt_generation,
        lease_expires_at_ns,
    });
    Ok(AsyncJobClaim::Acquired(AsyncJobAttempt {
        owner,
        attempt_generation,
        operation_generation: None,
        lease_expires_at_ns,
    }))
}

fn claim_replay_safe_attempt(
    current: &mut ReplaySafeAsyncAttemptFenceRecord,
    owner: AsyncJobOwner,
    now_ns: u64,
    lease_expires_at_ns: u64,
) -> Result<AsyncJobClaim, InternalError> {
    if let Some(active) = &current.active
        && active.lease_expires_at_ns > now_ns
    {
        return Ok(AsyncJobClaim::Busy {
            retry_at_ns: active.lease_expires_at_ns,
        });
    }
    let attempt_generation = current
        .last_attempt_generation
        .checked_add(1)
        .ok_or_else(InternalError::invariant)?;
    let operation_generation = if let Some(active) = &current.active {
        active.operation_generation
    } else if let Some(pending) = current.pending_operation_generation {
        pending
    } else {
        current
            .last_operation_generation
            .checked_add(1)
            .ok_or_else(InternalError::invariant)?
    };
    current.last_attempt_generation = attempt_generation;
    current.last_operation_generation = current.last_operation_generation.max(operation_generation);
    current.active = Some(ReplaySafeAsyncAttemptLeaseRecord {
        attempt_generation,
        operation_generation,
        lease_expires_at_ns,
    });
    Ok(AsyncJobClaim::Acquired(AsyncJobAttempt {
        owner,
        attempt_generation,
        operation_generation: Some(operation_generation),
        lease_expires_at_ns,
    }))
}

fn finish_attempt(current: &mut AsyncAttemptFenceRecord, attempt: AsyncJobAttempt) -> bool {
    let exact = attempt.operation_generation.is_none()
        && current
            .active
            .as_ref()
            .is_some_and(|active| active.attempt_generation == attempt.attempt_generation);
    if exact {
        current.active = None;
    }
    exact
}

fn finish_replay_safe_attempt(
    current: &mut ReplaySafeAsyncAttemptFenceRecord,
    attempt: AsyncJobAttempt,
    completion: AsyncJobCompletion,
) -> bool {
    let exact = current.active.as_ref().is_some_and(|active| {
        active.attempt_generation == attempt.attempt_generation
            && Some(active.operation_generation) == attempt.operation_generation
    });
    if !exact {
        return false;
    }
    current.active = None;
    current.pending_operation_generation = (completion == AsyncJobCompletion::RetryableFailure)
        .then_some(attempt.operation_generation)
        .flatten();
    true
}

fn lease_deadline(current: &AsyncAttemptFenceRecord) -> Option<u64> {
    current
        .active
        .as_ref()
        .map(|active| active.lease_expires_at_ns)
}
