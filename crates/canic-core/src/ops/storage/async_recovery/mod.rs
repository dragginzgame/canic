//! Module: ops::storage::async_recovery
//!
//! Responsibility: claim, finish, abandon, and inspect durable async timer attempts.
//! Does not own: timer registration, async domain work, retry timing, or provider state.
//! Boundary: workflows provide observed time and lease deadlines; ops commits exact fences.

#[cfg(test)]
mod tests;

use crate::{
    InternalError, InternalErrorOrigin,
    model::replay::OperationId,
    storage::stable::async_recovery::{
        AsyncRecoveryLeaseRecord, AsyncRecoveryOwnerRecord, AsyncRecoveryPendingScheduleRecord,
        AsyncTimerRecoveryRecord, AsyncTimerRecoveryStore,
    },
};
use sha2::{Digest, Sha256};

const ASYNC_RECOVERY_OPERATION_ID_DOMAIN: &[u8] = b"canic-async-recovery-operation:v1";

/// Closed identities for recovery-critical asynchronous timer owners.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AsyncRecoveryOwner {
    AuthRenewal,
    CanisterPoolMaintenance,
    CycleTopup,
    PlacementReceiptAcknowledgement,
}

impl AsyncRecoveryOwner {
    const fn discriminator(self) -> u8 {
        match self {
            Self::AuthRenewal => 1,
            Self::CanisterPoolMaintenance => 2,
            Self::CycleTopup => 3,
            Self::PlacementReceiptAcknowledgement => 4,
        }
    }
}

/// Exact durable attempt token. Only this token may finish its active lease.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsyncRecoveryAttempt {
    owner: AsyncRecoveryOwner,
    attempt_generation: u64,
    operation_generation: u64,
    lease_expires_at_ns: u64,
}

impl AsyncRecoveryAttempt {
    /// Return the deterministic operation identity retained across exact retries.
    #[must_use]
    pub fn operation_id(self) -> OperationId {
        let mut hasher = Sha256::new();
        hasher.update(ASYNC_RECOVERY_OPERATION_ID_DOMAIN);
        hasher.update([self.owner.discriminator()]);
        hasher.update(self.operation_generation.to_be_bytes());
        OperationId::from_bytes(hasher.finalize().into())
    }

    /// Return the authoritative lease deadline for coalesced scheduling.
    #[must_use]
    pub const fn lease_expires_at_ns(self) -> u64 {
        self.lease_expires_at_ns
    }
}

/// Result of attempting to enter one serial async recovery owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AsyncRecoveryClaim {
    Acquired(AsyncRecoveryAttempt),
    Busy { retry_at_ns: u64 },
}

/// Normal domain completion recorded independently from dispatcher delivery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AsyncRecoveryCompletion {
    Success,
    RetryableFailure,
    InvariantFailure,
}

/// Deterministic stable-state operations for serial async recovery attempts.
pub struct AsyncTimerRecoveryOps;

impl AsyncTimerRecoveryOps {
    /// Claim a fresh attempt, take over an expired attempt, or coalesce behind its lease.
    pub fn claim(
        owner: AsyncRecoveryOwner,
        now_ns: u64,
        lease_expires_at_ns: u64,
    ) -> Result<AsyncRecoveryClaim, InternalError> {
        if lease_expires_at_ns <= now_ns {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Ops,
                "async recovery lease deadline must be later than observed time",
            ));
        }
        let mut state = AsyncTimerRecoveryStore::get();
        let current = owner_record_mut(&mut state, owner);
        if let Some(active) = &current.active
            && active.lease_expires_at_ns > now_ns
        {
            return Ok(AsyncRecoveryClaim::Busy {
                retry_at_ns: active.lease_expires_at_ns,
            });
        }

        let attempt_generation = current
            .last_attempt_generation
            .checked_add(1)
            .ok_or_else(|| generation_exhausted("attempt"))?;
        let operation_generation = if let Some(active) = &current.active {
            active.operation_generation
        } else if let Some(pending) = current.pending_operation_generation {
            pending
        } else {
            current
                .last_operation_generation
                .checked_add(1)
                .ok_or_else(|| generation_exhausted("operation"))?
        };
        current.last_attempt_generation = attempt_generation;
        current.last_operation_generation =
            current.last_operation_generation.max(operation_generation);
        current.active = Some(AsyncRecoveryLeaseRecord {
            attempt_generation,
            operation_generation,
            lease_expires_at_ns,
        });
        if current.recovery_owned {
            current.recovery_due_at_ns = None;
        }
        AsyncTimerRecoveryStore::replace(state);
        Ok(AsyncRecoveryClaim::Acquired(AsyncRecoveryAttempt {
            owner,
            attempt_generation,
            operation_generation,
            lease_expires_at_ns,
        }))
    }

    /// Finish only the exact active attempt and optionally preserve its operation for retry.
    pub fn finish(
        attempt: AsyncRecoveryAttempt,
        completion: AsyncRecoveryCompletion,
        recovery_due_at_ns: Option<u64>,
    ) -> Result<bool, InternalError> {
        let mut state = AsyncTimerRecoveryStore::get();
        let current = owner_record_mut(&mut state, attempt.owner);
        let exact = current.active.as_ref().is_some_and(|active| {
            active.attempt_generation == attempt.attempt_generation
                && active.operation_generation == attempt.operation_generation
        });
        if !exact {
            return Ok(false);
        }
        current.active = None;
        let retry_pending = completion == AsyncRecoveryCompletion::RetryableFailure;
        current.pending_operation_generation =
            retry_pending.then_some(attempt.operation_generation);
        current.retry_streak = if retry_pending {
            current
                .retry_streak
                .checked_add(1)
                .ok_or_else(|| generation_exhausted("retry streak"))?
        } else {
            0
        };
        current.terminal_failure = completion == AsyncRecoveryCompletion::InvariantFailure;
        if current.recovery_owned {
            current.recovery_due_at_ns = match current.pending_schedule.take() {
                Some(AsyncRecoveryPendingScheduleRecord::Ensure(pending_due_at_ns)) => Some(
                    recovery_due_at_ns.map_or(pending_due_at_ns, |callback_due_at_ns| {
                        callback_due_at_ns.min(pending_due_at_ns)
                    }),
                ),
                Some(AsyncRecoveryPendingScheduleRecord::Reconcile(pending_due_at_ns)) => {
                    pending_due_at_ns
                }
                None => recovery_due_at_ns,
            };
        } else {
            current.pending_schedule = None;
        }
        AsyncTimerRecoveryStore::replace(state);
        Ok(true)
    }

    /// Transfer one stalled owner to the pre-armed watchdog's durable schedule.
    pub fn activate_recovery(owner: AsyncRecoveryOwner, due_at_ns: u64) {
        let mut state = AsyncTimerRecoveryStore::get();
        let current = owner_record_mut(&mut state, owner);
        current.recovery_owned = true;
        current.recovery_due_at_ns = Some(due_at_ns);
        if current.active.is_none() {
            current.pending_schedule = None;
        }
        AsyncTimerRecoveryStore::replace(state);
    }

    /// Retain an earliest scheduling request made while the provider still owns an attempt.
    pub(crate) fn record_active_ensure(owner: AsyncRecoveryOwner, due_at_ns: u64) {
        let mut state = AsyncTimerRecoveryStore::get();
        let current = owner_record_mut(&mut state, owner);
        if !current.recovery_owned && current.active.is_some() {
            current.pending_schedule =
                Some(merge_ensure(current.pending_schedule.take(), due_at_ns));
        }
        AsyncTimerRecoveryStore::replace(state);
    }

    /// Retain an authoritative scheduling request made during a provider-owned attempt.
    pub(crate) fn record_active_reconcile(owner: AsyncRecoveryOwner, due_at_ns: Option<u64>) {
        let mut state = AsyncTimerRecoveryStore::get();
        let current = owner_record_mut(&mut state, owner);
        if !current.recovery_owned && current.active.is_some() {
            current.pending_schedule =
                Some(AsyncRecoveryPendingScheduleRecord::Reconcile(due_at_ns));
        }
        AsyncTimerRecoveryStore::replace(state);
    }

    /// Reconcile a deadline only when the watchdog has taken over this owner.
    #[must_use]
    pub fn reconcile_recovery(owner: AsyncRecoveryOwner, due_at_ns: Option<u64>) -> bool {
        let mut state = AsyncTimerRecoveryStore::get();
        let current = owner_record_mut(&mut state, owner);
        if !current.recovery_owned {
            return false;
        }
        if current.active.is_some() {
            current.pending_schedule =
                Some(AsyncRecoveryPendingScheduleRecord::Reconcile(due_at_ns));
        } else {
            current.recovery_due_at_ns = due_at_ns;
        }
        AsyncTimerRecoveryStore::replace(state);
        true
    }

    /// Preserve the earliest requested deadline when watchdog scheduling is active.
    #[must_use]
    pub fn ensure_recovery(owner: AsyncRecoveryOwner, due_at_ns: u64) -> bool {
        let mut state = AsyncTimerRecoveryStore::get();
        let current = owner_record_mut(&mut state, owner);
        if !current.recovery_owned {
            return false;
        }
        if current.active.is_some() {
            current.pending_schedule =
                Some(merge_ensure(current.pending_schedule.take(), due_at_ns));
        } else {
            current.recovery_due_at_ns = Some(
                current
                    .recovery_due_at_ns
                    .map_or(due_at_ns, |current_due| current_due.min(due_at_ns)),
            );
        }
        AsyncTimerRecoveryStore::replace(state);
        true
    }

    /// Return whether bounded takeover moved this owner to watchdog scheduling.
    #[must_use]
    pub fn recovery_owned(owner: AsyncRecoveryOwner) -> bool {
        let mut state = AsyncTimerRecoveryStore::get();
        owner_record_mut(&mut state, owner).recovery_owned
    }

    /// Return one watchdog-owned deadline that is now due.
    #[must_use]
    pub fn recovery_due(owner: AsyncRecoveryOwner, now_ns: u64) -> Option<u64> {
        let mut state = AsyncTimerRecoveryStore::get();
        let current = owner_record_mut(&mut state, owner);
        current
            .recovery_owned
            .then_some(current.recovery_due_at_ns)
            .flatten()
            .filter(|due_at_ns| *due_at_ns <= now_ns)
    }

    /// Return the durable retry streak from completed domain attempts.
    #[must_use]
    pub fn retry_streak(owner: AsyncRecoveryOwner) -> u64 {
        let mut state = AsyncTimerRecoveryStore::get();
        owner_record_mut(&mut state, owner).retry_streak
    }

    /// Return whether the latest completed domain attempt failed terminally.
    #[must_use]
    pub fn is_terminal_failure(owner: AsyncRecoveryOwner) -> bool {
        let mut state = AsyncTimerRecoveryStore::get();
        owner_record_mut(&mut state, owner).terminal_failure
    }

    /// Return the active durable lease deadline for snapshot admission checks.
    #[must_use]
    pub fn active_lease_deadline(owner: AsyncRecoveryOwner) -> Option<u64> {
        let mut state = AsyncTimerRecoveryStore::get();
        owner_record_mut(&mut state, owner)
            .active
            .as_ref()
            .map(|active| active.lease_expires_at_ns)
    }

    /// Clear active and pending work when the domain owner is deliberately stopped.
    pub fn abandon(owner: AsyncRecoveryOwner) {
        let mut state = AsyncTimerRecoveryStore::get();
        let current = owner_record_mut(&mut state, owner);
        current.active = None;
        current.pending_operation_generation = None;
        current.pending_schedule = None;
        current.recovery_due_at_ns = None;
        current.recovery_owned = false;
        current.retry_streak = 0;
        current.terminal_failure = false;
        AsyncTimerRecoveryStore::replace(state);
    }

    /// Return the expired active-lease deadline that requires a serial re-kick.
    #[must_use]
    pub fn expired_deadline(owner: AsyncRecoveryOwner, now_ns: u64) -> Option<u64> {
        let mut state = AsyncTimerRecoveryStore::get();
        owner_record_mut(&mut state, owner)
            .active
            .as_ref()
            .filter(|active| active.lease_expires_at_ns <= now_ns)
            .map(|active| active.lease_expires_at_ns)
    }
}

const fn owner_record_mut(
    state: &mut AsyncTimerRecoveryRecord,
    owner: AsyncRecoveryOwner,
) -> &mut AsyncRecoveryOwnerRecord {
    match owner {
        AsyncRecoveryOwner::AuthRenewal => &mut state.auth_renewal,
        AsyncRecoveryOwner::CanisterPoolMaintenance => &mut state.canister_pool_maintenance,
        AsyncRecoveryOwner::CycleTopup => &mut state.cycle_topup,
        AsyncRecoveryOwner::PlacementReceiptAcknowledgement => {
            &mut state.placement_receipt_acknowledgement
        }
    }
}

fn generation_exhausted(kind: &str) -> InternalError {
    InternalError::invariant(
        InternalErrorOrigin::Ops,
        format!("async recovery {kind} generation is exhausted"),
    )
}

fn merge_ensure(
    current: Option<AsyncRecoveryPendingScheduleRecord>,
    due_at_ns: u64,
) -> AsyncRecoveryPendingScheduleRecord {
    match current {
        Some(AsyncRecoveryPendingScheduleRecord::Ensure(current_due_at_ns)) => {
            AsyncRecoveryPendingScheduleRecord::Ensure(current_due_at_ns.min(due_at_ns))
        }
        Some(AsyncRecoveryPendingScheduleRecord::Reconcile(current_due_at_ns)) => {
            AsyncRecoveryPendingScheduleRecord::Ensure(match current_due_at_ns {
                Some(current_due_at_ns) => current_due_at_ns.min(due_at_ns),
                None => due_at_ns,
            })
        }
        None => AsyncRecoveryPendingScheduleRecord::Ensure(due_at_ns),
    }
}
