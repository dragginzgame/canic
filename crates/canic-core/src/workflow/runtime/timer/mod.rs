//! Module: workflow::runtime::timer
//!
//! Responsibility: initialize the shared timer provider and coordinate authority suspension.
//! Does not own: domain schedules, recurrence, provider state, or control-plane callbacks.
//! Boundary: exact domain owners retain native claims; lifecycle uses detached native once work.

use crate::{
    InternalError,
    ops::{
        ic::IcOps,
        runtime::env::EnvOps,
        storage::async_job_recovery::{AsyncJobOwner, AsyncJobRecoveryOps},
    },
    workflow::{placement::acknowledgement::PlacementAcknowledgementWorkflow, runtime},
};
use ic_timers::{
    DeclarationLifetime, OnceContext, OnceRegistration, ScheduleError, TimerCadence,
    TimerCompletion, TimerDirective, TimerError as ProviderError, TimerIdentity,
    TimerIdentityError, TimerReconcileState, TimerRegistrationStatus, TimerRunResult,
    TimerSchedule, TimerSnapshot, WatchdogRegistration, WatchdogRunResult, initialize_runtime,
    reconcile_watchdog, register_once, timer_inventory,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    future::Future,
    thread::LocalKey,
    time::Duration,
};
use thiserror::Error;

const RECOVERY_WATCHDOG_CADENCE: Duration = Duration::from_secs(30);

thread_local! {
    static CORE_RECOVERY_WATCHDOG: RefCell<Option<WatchdogRegistration>> = const { RefCell::new(None) };
    static NEXT_LIFECYCLE_ID: Cell<u64> = const { Cell::new(0) };
    static TIMERS_SUSPENDED: Cell<bool> = const { Cell::new(false) };
}

/// Failure from Canic's bounded native custody, suspension, or lifecycle coordination.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TimerError {
    #[error("Canic timer claim custody is already borrowed")]
    CustodyBusy,
    #[error("Canic lifecycle timer identity allocation is exhausted")]
    LifecycleIdentityExhausted,
    #[error(transparent)]
    Identity(#[from] TimerIdentityError),
    #[error("Canic timer claim is missing")]
    MissingClaim,
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("timer registration rollback failed after {primary}: {cleanup}")]
    RegistrationRollback {
        primary: Box<Self>,
        cleanup: Box<Self>,
    },
    #[error("Canic timer claim is running and cannot be suspended: {0}")]
    RunningClaim(String),
    #[error(transparent)]
    Schedule(#[from] ScheduleError),
    #[error("Canic timers are suspended for an authority snapshot")]
    Suspended,
    #[error("authority snapshots do not support a timer outside Canic custody: {0}")]
    UnmanagedClaim(String),
    #[error("Canic timer claim has the wrong scheduling policy")]
    WrongPolicy,
}

impl From<TimerError> for InternalError {
    fn from(_error: TimerError) -> Self {
        Self::invariant()
    }
}

/// Authority coordination over exact native timer owners.
pub struct TimerAuthorityWorkflow;

impl TimerAuthorityWorkflow {
    /// Initialize only the shared timer runtime for a canister with no declared jobs yet.
    pub(crate) fn initialize_shared_runtime() -> Result<(), TimerError> {
        initialize_runtime()?;
        Ok(())
    }

    /// Initialize the shared runtime; non-root claims remain lazy and domain-owned.
    pub(crate) fn initialize_nonroot_runtime() -> Result<(), TimerError> {
        Self::initialize_shared_runtime()
    }

    /// Initialize the shared runtime; the control plane declares Root-only claims later.
    pub(crate) fn initialize_root_runtime() -> Result<(), TimerError> {
        Self::initialize_shared_runtime()
    }

    /// Restore volatile suspension from the durable authority fence.
    pub(crate) fn restore_snapshot_suspension(sealed: bool) {
        TIMERS_SUSPENDED.with(|suspended| suspended.set(sealed));
    }

    /// Return whether durable lifecycle restoration left timer owners suspended.
    #[must_use]
    pub(crate) fn is_suspended() -> bool {
        TIMERS_SUSPENDED.with(Cell::get)
    }

    /// Arm the one non-root watchdog after an exact domain owner reconstructs demand.
    pub(crate) fn ensure_async_job_recovery_watchdog() -> Result<(), TimerError> {
        require_active()?;
        if EnvOps::is_root() {
            return Ok(());
        }
        reconcile_core_recovery_watchdog(
            TimerReconcileState::Scheduled,
            Self::recover_expired_async_jobs,
        )
    }

    /// Arm the same watchdog with the active role's automatic top-up recovery owner.
    pub(crate) fn ensure_async_job_recovery_watchdog_with_automatic_topup() -> Result<(), TimerError>
    {
        require_active()?;
        reconcile_core_recovery_watchdog(
            TimerReconcileState::Scheduled,
            Self::recover_expired_async_jobs_with_automatic_topup,
        )
    }

    /// Prove that every Root-owned timer and business attempt can be suspended.
    pub(crate) fn require_root_resumable() -> Result<(), TimerError> {
        require_no_active_async_job_attempts()?;
        let mut identities = BTreeSet::from([
            canister_pool_timer_identity()?,
            recovery_watchdog_identity()?,
        ]);
        for identity in [
            runtime::auth::RuntimeAuthWorkflow::claimed_root_issuer_renewal_timer_identity()?,
            runtime::intent::IntentCleanupWorkflow::claimed_timer_identity()?,
            runtime::log::LogRetentionWorkflow::claimed_timer_identity()?,
            runtime::cycles::CycleWorkflow::claimed_timer_identity()?,
            PlacementAcknowledgementWorkflow::claimed_timer_identity()?,
            claimed_core_recovery_watchdog_identity()?,
        ]
        .into_iter()
        .flatten()
        {
            identities.insert(identity);
        }
        require_observed_claims_resumable(
            &identities,
            timer_inventory()?
                .into_timers()
                .into_iter()
                .map(|snapshot| (snapshot.identity().clone(), snapshot.registration_status())),
        )
    }

    /// Prove that Coordinator has no live private lifecycle work to snapshot.
    pub(crate) fn require_coordinator_resumable() -> Result<(), TimerError> {
        require_observed_claims_resumable(
            &BTreeSet::new(),
            timer_inventory()?
                .into_timers()
                .into_iter()
                .map(|snapshot| (snapshot.identity().clone(), snapshot.registration_status())),
        )
    }

    /// Disarm exact Root core-owned claims without affecting another provider owner.
    pub(crate) fn suspend_root() -> Result<(), TimerError> {
        Self::require_root_resumable()?;
        TIMERS_SUSPENDED.with(|suspended| suspended.set(true));

        runtime::auth::RuntimeAuthWorkflow::cancel_root_issuer_renewal_timer()?;
        runtime::intent::IntentCleanupWorkflow::cancel_timer()?;
        runtime::log::LogRetentionWorkflow::cancel_timer()?;
        runtime::cycles::CycleWorkflow::cancel_timer()?;
        PlacementAcknowledgementWorkflow::cancel_timer()?;
        cancel_core_recovery_watchdog()?;
        Ok(())
    }

    /// Seal a Coordinator only when its native inventory is empty.
    pub(crate) fn suspend_coordinator() -> Result<(), TimerError> {
        Self::require_coordinator_resumable()?;
        TIMERS_SUSPENDED.with(|suspended| suspended.set(true));
        Ok(())
    }

    /// End Root suspension before exact domain owners reconstruct current demand.
    pub(crate) fn resume_root() {
        TIMERS_SUSPENDED.with(|suspended| suspended.set(false));
    }

    /// End Coordinator suspension; it owns no fixed background claims.
    pub(crate) fn resume_coordinator() {
        TIMERS_SUSPENDED.with(|suspended| suspended.set(false));
    }

    /// Schedule one private lifecycle deferral as a direct remove-on-stop native claim.
    pub(crate) fn defer_lifecycle_once(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> Result<(), TimerError> {
        register_lifecycle_once(delay, label.into(), async move {
            task.await;
            TimerRunResult::new(TimerCompletion::success(1), TimerDirective::Stop)
        })
    }

    /// Schedule lifecycle work whose typed completion must remain observable.
    pub(crate) fn defer_lifecycle_result_once(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = TimerRunResult> + 'static,
    ) -> Result<(), TimerError> {
        register_lifecycle_once(delay, label.into(), task)
    }

    /// Recover expired core-owned business attempts for the one role-native watchdog.
    pub(crate) fn recover_expired_async_jobs(now_ns: u64) -> u64 {
        let mut recovered = 0u64;
        if runtime::auth::RuntimeAuthWorkflow::recover_expired_root_issuer_renewal(now_ns) {
            recovered = recovered.saturating_add(1);
        }
        if PlacementAcknowledgementWorkflow::recover_expired_timer(now_ns) {
            recovered = recovered.saturating_add(1);
        }
        recovered
    }

    /// Recover the base set plus automatic top-up for a capability-bearing non-root.
    pub(crate) fn recover_expired_async_jobs_with_automatic_topup(now_ns: u64) -> u64 {
        let recovered = Self::recover_expired_async_jobs(now_ns);
        if runtime::cycles::CycleWorkflow::recover_expired_timer(now_ns) {
            return recovered.saturating_add(1);
        }
        recovered
    }

    /// Return the shared canonical timer inventory in deterministic identity order.
    pub fn statuses() -> Result<Vec<TimerSnapshot>, TimerError> {
        Ok(timer_inventory()?.into_timers())
    }
}

fn require_no_active_async_job_attempts() -> Result<(), TimerError> {
    let owners = [
        (
            AsyncJobOwner::AuthRenewal,
            runtime::auth::RuntimeAuthWorkflow::root_issuer_renewal_timer_identity()?,
        ),
        (
            AsyncJobOwner::PlacementReceiptAcknowledgement,
            PlacementAcknowledgementWorkflow::timer_identity()?,
        ),
        (
            AsyncJobOwner::CanisterPoolMaintenance,
            canister_pool_timer_identity()?,
        ),
        (
            AsyncJobOwner::CycleTopup,
            runtime::cycles::CycleWorkflow::timer_identity()?,
        ),
    ];
    for (owner, identity) in owners {
        if AsyncJobRecoveryOps::active_lease_deadline(owner).is_some() {
            return Err(TimerError::RunningClaim(format_identity(&identity)));
        }
    }
    Ok(())
}

fn reconcile_core_recovery_watchdog(
    desired: TimerReconcileState,
    recover: fn(u64) -> u64,
) -> Result<(), TimerError> {
    let identity = recovery_watchdog_identity()?;
    let cadence = TimerCadence::new(RECOVERY_WATCHDOG_CADENCE)?;
    CORE_RECOVERY_WATCHDOG
        .try_with(|registration| {
            let mut registration = registration
                .try_borrow_mut()
                .map_err(|_| TimerError::CustodyBusy)?;
            reconcile_watchdog(
                &mut registration,
                &identity,
                cadence,
                desired,
                move |_context| run_core_recovery_watchdog(recover),
            )
            .map_err(TimerError::from)
        })
        .map_err(|_| TimerError::CustodyBusy)?
}

fn claimed_core_recovery_watchdog_identity() -> Result<Option<TimerIdentity>, TimerError> {
    CORE_RECOVERY_WATCHDOG
        .try_with(|registration| {
            let registration = registration
                .try_borrow()
                .map_err(|_| TimerError::CustodyBusy)?;
            Ok(registration
                .as_ref()
                .map(|registration| registration.identity().clone()))
        })
        .map_err(|_| TimerError::CustodyBusy)?
}

fn cancel_core_recovery_watchdog() -> Result<(), TimerError> {
    CORE_RECOVERY_WATCHDOG
        .try_with(|registration| {
            let registration = registration
                .try_borrow()
                .map_err(|_| TimerError::CustodyBusy)?;
            if let Some(registration) = registration.as_ref() {
                registration.cancel()?;
            }
            Ok(())
        })
        .map_err(|_| TimerError::CustodyBusy)?
}

fn run_core_recovery_watchdog(recover: fn(u64) -> u64) -> WatchdogRunResult {
    let recovered = recover(IcOps::now_nanos());
    let completion = if recovered == 0 {
        TimerCompletion::no_work()
    } else {
        TimerCompletion::success(recovered)
    };
    WatchdogRunResult::new(completion, ic_timers::WatchdogDecision::Continue)
}

pub fn recovery_watchdog_identity() -> Result<TimerIdentity, TimerError> {
    TimerIdentity::try_new("canic", "async_job_recovery", "watchdog").map_err(Into::into)
}

fn canister_pool_timer_identity() -> Result<TimerIdentity, TimerError> {
    TimerIdentity::try_new("canic", "canister_pool", "maintain").map_err(Into::into)
}

fn register_lifecycle_once(
    delay: Duration,
    label: String,
    task: impl Future<Output = TimerRunResult> + 'static,
) -> Result<(), TimerError> {
    require_active()?;
    let identity = next_lifecycle_identity(label)?;
    let mut task = Some(task);
    let registration = register_once(
        identity,
        DeclarationLifetime::RemoveWhenStopped,
        move |_context: OnceContext| {
            let task = task.take();
            async move {
                match task {
                    Some(task) => task.await,
                    None => TimerRunResult::new(
                        TimerCompletion::invariant_failure(0),
                        TimerDirective::Stop,
                    ),
                }
            }
        },
    )?;
    if let Err(primary) = registration.ensure_scheduled(TimerSchedule::After(delay)) {
        return match registration.unregister() {
            Ok(()) => Err(primary.into()),
            Err(cleanup) => Err(TimerError::RegistrationRollback {
                primary: Box::new(primary.into()),
                cleanup: Box::new(cleanup.into()),
            }),
        };
    }
    drop(registration);
    Ok(())
}

fn next_lifecycle_identity(label: String) -> Result<TimerIdentity, TimerError> {
    let id = NEXT_LIFECYCLE_ID.with(|next| {
        let id = next
            .get()
            .checked_add(1)
            .ok_or(TimerError::LifecycleIdentityExhausted)?;
        next.set(id);
        Ok::<_, TimerError>(id)
    })?;
    TimerIdentity::try_new("canic", format!("lifecycle-{id}"), label).map_err(Into::into)
}

pub fn require_active() -> Result<(), TimerError> {
    if TIMERS_SUSPENDED.with(Cell::get) {
        return Err(TimerError::Suspended);
    }
    Ok(())
}

/// Borrow one exact domain owner's native once registration without moving custody.
pub fn with_owned_once<T>(
    owner: &'static LocalKey<RefCell<Option<OnceRegistration>>>,
    operation: impl FnOnce(&OnceRegistration) -> T,
) -> Result<Option<T>, TimerError> {
    owner
        .try_with(|registration| {
            let registration = registration
                .try_borrow()
                .map_err(|_| TimerError::CustodyBusy)?;
            Ok::<_, TimerError>(registration.as_ref().map(operation))
        })
        .map_err(|_| TimerError::CustodyBusy)?
}

/// Retain one exact domain owner's native once registration with rollback on rejection.
pub fn retain_owned_once(
    owner: &'static LocalKey<RefCell<Option<OnceRegistration>>>,
    registration: OnceRegistration,
) -> Result<(), TimerError> {
    retain_with_rollback(
        registration,
        |registration| {
            owner.with(|current| {
                let Ok(mut current) = current.try_borrow_mut() else {
                    return Err((TimerError::CustodyBusy, registration));
                };
                if current.is_some() {
                    return Err((TimerError::WrongPolicy, registration));
                }
                *current = Some(registration);
                Ok(())
            })
        },
        |registration| registration.unregister().map_err(TimerError::from),
    )
}

fn retain_with_rollback<T>(
    claim: T,
    retain: impl FnOnce(T) -> Result<(), (TimerError, T)>,
    cleanup: impl FnOnce(T) -> Result<(), TimerError>,
) -> Result<(), TimerError> {
    match retain(claim) {
        Ok(()) => Ok(()),
        Err((primary, claim)) => match cleanup(claim) {
            Ok(()) => Err(primary),
            Err(cleanup) => Err(TimerError::RegistrationRollback {
                primary: Box::new(primary),
                cleanup: Box::new(cleanup),
            }),
        },
    }
}

fn format_identity(identity: &TimerIdentity) -> String {
    format!(
        "{}/{}/{}",
        identity.owner(),
        identity.subsystem(),
        identity.name()
    )
}

fn require_observed_claims_resumable(
    claimed: &BTreeSet<TimerIdentity>,
    observed: impl IntoIterator<Item = (TimerIdentity, TimerRegistrationStatus)>,
) -> Result<(), TimerError> {
    for (identity, registration) in observed {
        if !claimed.contains(&identity) {
            return Err(TimerError::UnmanagedClaim(format_identity(&identity)));
        }
        if registration == TimerRegistrationStatus::Running {
            return Err(TimerError::RunningClaim(format_identity(&identity)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::storage::async_job_recovery::AsyncJobClaim;
    use std::{cell::Cell, collections::BTreeSet};

    #[test]
    fn fixed_claim_identities_are_exact_and_unique() {
        let identities = [
            runtime::intent::IntentCleanupWorkflow::timer_identity()
                .expect("intent cleanup identity"),
            runtime::log::LogRetentionWorkflow::timer_identity().expect("log retention identity"),
            runtime::auth::RuntimeAuthWorkflow::root_issuer_renewal_timer_identity()
                .expect("auth renewal identity"),
            runtime::cycles::CycleWorkflow::timer_identity().expect("cycle top-up identity"),
            PlacementAcknowledgementWorkflow::timer_identity()
                .expect("placement acknowledgement identity"),
            recovery_watchdog_identity().expect("recovery watchdog identity"),
            canister_pool_timer_identity().expect("canister pool identity"),
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();

        assert_eq!(identities.len(), 7);
        assert!(
            identities
                .iter()
                .all(|identity| identity.owner() == "canic")
        );
    }

    #[test]
    fn failed_custody_insertion_runs_registration_cleanup() {
        let cleaned = Cell::new(false);
        let error = retain_with_rollback(
            17u8,
            |claim| Err((TimerError::CustodyBusy, claim)),
            |claim| {
                assert_eq!(claim, 17);
                cleaned.set(true);
                Ok(())
            },
        )
        .expect_err("custody rejection must propagate");

        assert!(matches!(error, TimerError::CustodyBusy));
        assert!(cleaned.get());
    }

    #[test]
    fn authority_snapshot_rejects_a_claim_outside_canic_custody() {
        let external = TimerIdentity::try_new("companion-framework", "snapshot", "unmanaged")
            .expect("external identity");
        assert!(matches!(
            require_observed_claims_resumable(
                &BTreeSet::new(),
                [(external, TimerRegistrationStatus::Unregistered)]
            ),
            Err(TimerError::UnmanagedClaim(identity))
                if identity == "companion-framework/snapshot/unmanaged"
        ));
    }

    #[test]
    fn authority_snapshot_rejects_a_running_canic_claim() {
        let identity =
            TimerIdentity::try_new("canic", "cycles", "topup").expect("Canic timer identity");
        assert!(matches!(
            require_observed_claims_resumable(
                &BTreeSet::from([identity.clone()]),
                [(identity, TimerRegistrationStatus::Running)]
            ),
            Err(TimerError::RunningClaim(identity)) if identity == "canic/cycles/topup"
        ));
    }

    #[test]
    fn authority_snapshot_rejects_a_watchdog_dispatched_async_job_attempt() {
        let owner = AsyncJobOwner::CanisterPoolMaintenance;
        AsyncJobRecoveryOps::abandon(owner);
        assert!(matches!(
            AsyncJobRecoveryOps::claim(owner, 10, 20),
            Ok(AsyncJobClaim::Acquired(_))
        ));

        assert!(matches!(
            require_no_active_async_job_attempts(),
            Err(TimerError::RunningClaim(identity))
                if identity == "canic/canister_pool/maintain"
        ));
        AsyncJobRecoveryOps::abandon(owner);
    }
}
