//! Module: workflow::runtime::timer
//!
//! Responsibility: adapt Canic timer ownership to the shared `ic-timers` runtime.
//! Does not own: provider handles, timer arbitration, recurrence state, counters, or snapshots.
//! Boundary: Canic retains only bounded opaque registration claims; `ic-timers` is canonical.

use crate::{
    InternalError, InternalErrorOrigin,
    domain::runtime::TimerExecutionOutcome,
    workflow::{placement::acknowledgement::PlacementAcknowledgementWorkflow, runtime},
};
use ic_timers::{
    AfterCompletionContext, AfterCompletionRegistration, DeclarationLifetime, OnceContext,
    OnceRegistration, ScheduleError, TimerCadence, TimerCompletion,
    TimerDirective as ProviderDirective, TimerError as ProviderError, TimerIdentity,
    TimerIdentityError, TimerRegistrationStatus, TimerRunResult as ProviderRunResult,
    TimerSchedule, TimerSnapshot, initialize_runtime, register_after_completion, register_once,
    timer_snapshot, timer_snapshots,
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    future::Future,
    time::Duration,
};
use thiserror::Error;

type SnapshotResumeParticipant = fn() -> Result<(), TimerError>;

thread_local! {
    static CLAIMS: RefCell<BTreeMap<ClaimKey, TimerClaim>> = const { RefCell::new(BTreeMap::new()) };
    static NEXT_TRANSIENT_ID: Cell<u64> = const { Cell::new(0) };
    static SNAPSHOT_RESUME_PARTICIPANT: Cell<Option<SnapshotResumeParticipant>> = const { Cell::new(None) };
    static TIMERS_SUSPENDED: Cell<bool> = const { Cell::new(false) };
}

/// Closed identities for Canic-owned dynamic background processes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum TimerKey {
    AuthRenewal,
    CycleTopup,
    IntentCleanup,
    LogRetention,
    PlacementReceiptAcknowledgement,
}

impl TimerKey {
    #[cfg(test)]
    const ALL: [Self; 5] = [
        Self::AuthRenewal,
        Self::CycleTopup,
        Self::IntentCleanup,
        Self::LogRetention,
        Self::PlacementReceiptAcknowledgement,
    ];
    const NONROOT: [Self; 4] = [
        Self::CycleTopup,
        Self::IntentCleanup,
        Self::LogRetention,
        Self::PlacementReceiptAcknowledgement,
    ];
    const ROOT: [Self; 4] = [
        Self::AuthRenewal,
        Self::CycleTopup,
        Self::IntentCleanup,
        Self::LogRetention,
    ];

    const fn identity_parts(self) -> (&'static str, &'static str) {
        match self {
            Self::AuthRenewal => ("auth_renewal", "run"),
            Self::CycleTopup => ("cycles", "topup"),
            Self::IntentCleanup => ("intent_cleanup", "run"),
            Self::LogRetention => ("log_retention", "run"),
            Self::PlacementReceiptAcknowledgement => ("placement", "receipt_ack"),
        }
    }

    fn identity(self) -> Result<TimerIdentity, TimerError> {
        let (subsystem, name) = self.identity_parts();
        TimerIdentity::try_new("canic", subsystem, name).map_err(Into::into)
    }
}

/// Scheduling decision returned after one bounded built-in invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerDirective {
    Stop,
    ContinueImmediately,
    RetryAfter(Duration),
    ScheduleAt(u64),
}

/// Typed result of one bounded built-in invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerRunResult {
    pub outcome: TimerExecutionOutcome,
    pub work_count: u64,
    pub directive: TimerDirective,
}

impl TimerRunResult {
    #[must_use]
    pub const fn success(work_count: u64, directive: TimerDirective) -> Self {
        Self {
            outcome: TimerExecutionOutcome::Success,
            work_count,
            directive,
        }
    }

    #[must_use]
    pub const fn no_work(directive: TimerDirective) -> Self {
        Self {
            outcome: TimerExecutionOutcome::NoWork,
            work_count: 0,
            directive,
        }
    }

    #[must_use]
    pub const fn invariant_failure() -> Self {
        Self {
            outcome: TimerExecutionOutcome::InvariantFailure,
            work_count: 0,
            directive: TimerDirective::Stop,
        }
    }

    const fn into_provider(self) -> ProviderRunResult {
        let completion = match self.outcome {
            TimerExecutionOutcome::Success => TimerCompletion::success(self.work_count),
            TimerExecutionOutcome::NoWork => TimerCompletion::no_work(),
            TimerExecutionOutcome::RetryableFailure => {
                TimerCompletion::retryable_failure(self.work_count)
            }
            TimerExecutionOutcome::InvariantFailure | TimerExecutionOutcome::Unacknowledged => {
                TimerCompletion::invariant_failure(self.work_count)
            }
        };
        let directive = match self.directive {
            TimerDirective::Stop => ProviderDirective::Stop,
            TimerDirective::ContinueImmediately => ProviderDirective::ContinueImmediately,
            TimerDirective::RetryAfter(delay) => ProviderDirective::RetryAfter(delay),
            TimerDirective::ScheduleAt(deadline_ns) => ProviderDirective::ScheduleAt(deadline_ns),
        };
        ProviderRunResult::new(completion, directive)
    }
}

/// Failure from Canic's public timer adapter or bounded claim custody.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TimerError {
    #[error("Canic timer claim custody is already borrowed")]
    CustodyBusy,
    #[error("Canic timer identity allocation is exhausted")]
    IdentityExhausted,
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
    fn from(error: TimerError) -> Self {
        Self::invariant(
            InternalErrorOrigin::Workflow,
            format!("Canic timer runtime failed: {error}"),
        )
    }
}

/// Opaque identity returned to the public timer facade.
#[derive(Debug, Eq, PartialEq)]
pub struct TimerClaimId(ClaimKey);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ClaimKey {
    BuiltIn(TimerKey),
    CanisterPoolMaintenance,
    Transient(u64),
}

enum TimerClaim {
    AfterCompletion(AfterCompletionRegistration),
    Once(OnceRegistration),
}

impl TimerClaim {
    const fn identity(&self) -> &TimerIdentity {
        match self {
            Self::AfterCompletion(registration) => registration.identity(),
            Self::Once(registration) => registration.identity(),
        }
    }

    fn cancel(&self) -> Result<(), TimerError> {
        match self {
            Self::AfterCompletion(registration) => registration.cancel()?,
            Self::Once(registration) => registration.cancel()?,
        }
        Ok(())
    }

    fn unregister(self) -> Result<(), TimerError> {
        match self {
            Self::AfterCompletion(registration) => registration.unregister()?,
            Self::Once(registration) => registration.unregister()?,
        }
        Ok(())
    }
}

/// Canonical Canic adapter for the shared canister-local timer runtime.
pub struct TimerWorkflow;

impl TimerWorkflow {
    /// Initialize only the shared timer runtime for a canister with no Canic runtime jobs.
    pub(crate) fn initialize_shared_runtime() -> Result<(), TimerError> {
        initialize_runtime()?;
        Ok(())
    }

    /// Initialize the shared runtime and reserve fixed non-root declarations.
    pub(crate) fn initialize_nonroot_runtime() -> Result<(), TimerError> {
        Self::initialize_shared_runtime()?;
        for key in TimerKey::NONROOT {
            declare_builtin(key)?;
        }
        Ok(())
    }

    /// Initialize the shared runtime and reserve fixed root declarations.
    pub(crate) fn initialize_root_runtime() -> Result<(), TimerError> {
        Self::initialize_shared_runtime()?;
        for key in TimerKey::ROOT {
            declare_builtin(key)?;
        }
        Ok(())
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

    /// Register the one root control-plane timer reconciler for snapshot resume.
    pub(crate) fn register_snapshot_resume_participant(
        participant: fn() -> Result<(), TimerError>,
    ) {
        SNAPSHOT_RESUME_PARTICIPANT.with(|current| current.set(Some(participant)));
    }

    /// Disarm every Canic-owned claim without affecting other runtime owners.
    pub(crate) fn suspend_all() -> Result<(), TimerError> {
        Self::require_resumable()?;
        TIMERS_SUSPENDED.with(|suspended| suspended.set(true));

        let transient_keys = CLAIMS
            .try_with(|claims| {
                let claims = claims.try_borrow().map_err(|_| TimerError::CustodyBusy)?;
                Ok::<_, TimerError>(
                    claims
                        .keys()
                        .copied()
                        .filter(|key| matches!(key, ClaimKey::Transient(_)))
                        .collect::<Vec<_>>(),
                )
            })
            .map_err(|_| TimerError::CustodyBusy)??;

        CLAIMS
            .try_with(|claims| {
                let claims = claims.try_borrow().map_err(|_| TimerError::CustodyBusy)?;
                for (key, claim) in claims.iter() {
                    if !matches!(key, ClaimKey::Transient(_)) {
                        claim.cancel()?;
                    }
                }
                Ok::<_, TimerError>(())
            })
            .map_err(|_| TimerError::CustodyBusy)??;

        for key in transient_keys {
            remove_claim(key)?.unregister()?;
        }
        Ok(())
    }

    /// Prove that no Canic-owned claim is currently executing.
    pub(crate) fn require_resumable() -> Result<(), TimerError> {
        let identities = CLAIMS
            .try_with(|claims| {
                let claims = claims.try_borrow().map_err(|_| TimerError::CustodyBusy)?;
                Ok::<_, TimerError>(
                    claims
                        .values()
                        .map(|claim| claim.identity().clone())
                        .collect::<BTreeSet<_>>(),
                )
            })
            .map_err(|_| TimerError::CustodyBusy)??;
        require_observed_claims_resumable(
            &identities,
            timer_snapshots()?
                .into_iter()
                .map(|snapshot| (snapshot.identity().clone(), snapshot.registration_status())),
        )
    }

    /// End snapshot suspension before domain owners reconstruct their deadlines.
    pub(crate) fn resume_all() -> Result<(), TimerError> {
        TIMERS_SUSPENDED.with(|suspended| suspended.set(false));
        if let Some(participant) = SNAPSHOT_RESUME_PARTICIPANT.with(Cell::get)
            && let Err(error) = participant()
        {
            TIMERS_SUSPENDED.with(|suspended| suspended.set(true));
            return Err(error);
        }
        Ok(())
    }

    /// Schedule a cancellable application one-shot.
    pub fn set_application_once(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> Result<TimerClaimId, TimerError> {
        register_transient_once("application", delay, label.into(), task)
    }

    /// Schedule one cancellable lifecycle deferral.
    pub fn set_lifecycle_once(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> Result<TimerClaimId, TimerError> {
        register_transient_once("lifecycle", delay, label.into(), task)
    }

    /// Schedule lifecycle work whose typed completion must remain observable.
    pub fn set_lifecycle_result_once(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = TimerRunResult> + 'static,
    ) -> Result<TimerClaimId, TimerError> {
        register_transient_result_once("lifecycle", delay, label.into(), task)
    }

    /// Schedule a cancellable, non-overlapping after-completion interval.
    pub fn set_application_interval<F, Fut>(
        interval: Duration,
        label: impl Into<String>,
        mut task: F,
    ) -> Result<TimerClaimId, TimerError>
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        require_active()?;
        discard_expired_transient_claims()?;
        let (key, identity) = next_transient_identity("application", label.into())?;
        let cadence = TimerCadence::new(interval)?;
        let registration = register_after_completion(
            identity,
            cadence,
            DeclarationLifetime::RemoveWhenStopped,
            move |_context: AfterCompletionContext| {
                let future = task();
                async move {
                    future.await;
                    ProviderRunResult::new(
                        TimerCompletion::success(1),
                        ProviderDirective::RecurAfterCompletion,
                    )
                }
            },
        )?;
        if let Err(error) = registration.ensure_scheduled() {
            return Err(rollback_registration(
                TimerClaim::AfterCompletion(registration),
                error.into(),
            ));
        }
        retain_claim(key, TimerClaim::AfterCompletion(registration))?;
        Ok(TimerClaimId(key))
    }

    /// Declare or re-arm the fixed root canister-pool maintenance cadence.
    #[doc(hidden)]
    pub fn set_canister_pool_maintenance<F, Fut>(
        interval: Duration,
        task: F,
    ) -> Result<TimerClaimId, TimerError>
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = TimerRunResult> + 'static,
    {
        require_active()?;
        let key = ClaimKey::CanisterPoolMaintenance;
        let handle = Self::declare_canister_pool_maintenance(interval, task)?;
        with_claim(key, |claim| match claim {
            TimerClaim::AfterCompletion(registration) => {
                registration.ensure_scheduled().map_err(TimerError::from)
            }
            TimerClaim::Once(_) => Err(TimerError::WrongPolicy),
        })?
        .ok_or(TimerError::MissingClaim)??;
        Ok(handle)
    }

    /// Reserve inactive root canister-pool maintenance before application hooks run.
    pub fn declare_canister_pool_maintenance<F, Fut>(
        interval: Duration,
        mut task: F,
    ) -> Result<TimerClaimId, TimerError>
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = TimerRunResult> + 'static,
    {
        require_active()?;
        let key = ClaimKey::CanisterPoolMaintenance;
        if with_claim(key, |_| ())?.is_some() {
            return Ok(TimerClaimId(key));
        }

        let identity = TimerIdentity::try_new("canic", "canister_pool", "maintain")?;
        let cadence = TimerCadence::new(interval)?;
        let registration = register_after_completion(
            identity,
            cadence,
            DeclarationLifetime::Retained,
            move |_context: AfterCompletionContext| {
                let future = task();
                async move {
                    let result = future.await.into_provider();
                    ProviderRunResult::new(
                        result.completion(),
                        ProviderDirective::RecurAfterCompletion,
                    )
                }
            },
        )?;
        retain_claim(key, TimerClaim::AfterCompletion(registration))?;
        Ok(TimerClaimId(key))
    }

    /// Consume a public handle and suppress future invocations.
    pub fn cancel(handle: TimerClaimId) -> Result<(), TimerError> {
        match handle.0 {
            ClaimKey::Transient(_) => remove_claim(handle.0)?.unregister()?,
            ClaimKey::CanisterPoolMaintenance => {
                with_claim(handle.0, TimerClaim::cancel)?.ok_or(TimerError::MissingClaim)??;
            }
            ClaimKey::BuiltIn(_) => return Err(TimerError::WrongPolicy),
        }
        Ok(())
    }

    /// Request the earliest desired run for one fixed built-in.
    pub fn schedule(key: TimerKey, delay: Duration) -> Result<(), TimerError> {
        require_active()?;
        with_builtin(key, |registration| {
            registration.ensure_scheduled(TimerSchedule::After(delay))
        })
    }

    /// Request the earliest desired absolute deadline for one fixed built-in.
    pub fn schedule_at(key: TimerKey, deadline_ns: u64) -> Result<(), TimerError> {
        require_active()?;
        with_builtin(key, |registration| {
            registration.ensure_scheduled(TimerSchedule::At(deadline_ns))
        })
    }

    /// Reconcile one fixed built-in to its exact authoritative deadline.
    pub fn reconcile_at(key: TimerKey, deadline_ns: Option<u64>) -> Result<(), TimerError> {
        require_active()?;
        with_builtin(key, |registration| {
            registration.reconcile_schedule(deadline_ns.map(TimerSchedule::At))
        })
    }

    /// Return the shared canonical timer inventory in deterministic identity order.
    pub fn statuses() -> Result<Vec<TimerSnapshot>, TimerError> {
        Ok(timer_snapshots()?)
    }

    /// Return the completed retryable-failure streak for one built-in owner.
    #[must_use]
    pub(crate) fn consecutive_expected_failures(key: TimerKey) -> u64 {
        key.identity()
            .and_then(|identity| Ok(ic_timers::consecutive_expected_failures(&identity)?))
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    /// Return whether one built-in owner stopped on a failure.
    #[must_use]
    pub(crate) fn is_failed(key: TimerKey) -> bool {
        key.identity()
            .and_then(|identity| Ok(timer_snapshot(&identity)?))
            .ok()
            .flatten()
            .is_some_and(|snapshot| {
                snapshot.process_condition() == ic_timers::TimerProcessCondition::Failed
            })
    }
}

fn declare_builtin(key: TimerKey) -> Result<(), TimerError> {
    let claim_key = ClaimKey::BuiltIn(key);
    if with_claim(claim_key, |_| ())?.is_some() {
        return Ok(());
    }
    let registration = register_once(
        key.identity()?,
        DeclarationLifetime::Retained,
        move |_context: OnceContext| async move { run_builtin(key).await.into_provider() },
    )?;
    retain_claim(claim_key, TimerClaim::Once(registration))
}

async fn run_builtin(key: TimerKey) -> TimerRunResult {
    match key {
        TimerKey::AuthRenewal => {
            runtime::auth::RuntimeAuthWorkflow::run_root_issuer_renewal_timer().await
        }
        TimerKey::CycleTopup => runtime::cycles::CycleWorkflow::run_topup().await,
        TimerKey::IntentCleanup => runtime::intent::IntentCleanupWorkflow::run_due_batch(),
        TimerKey::LogRetention => runtime::log::LogRetentionWorkflow::run_due_batch(),
        TimerKey::PlacementReceiptAcknowledgement => {
            PlacementAcknowledgementWorkflow::run_scheduled().await
        }
    }
}

fn register_transient_once(
    scope: &str,
    delay: Duration,
    label: String,
    task: impl Future<Output = ()> + 'static,
) -> Result<TimerClaimId, TimerError> {
    require_active()?;
    discard_expired_transient_claims()?;
    let (key, identity) = next_transient_identity(scope, label)?;
    let mut task = Some(task);
    let registration = register_once(
        identity,
        DeclarationLifetime::RemoveWhenStopped,
        move |_context: OnceContext| {
            let task = task.take();
            async move {
                if let Some(task) = task {
                    task.await;
                }
                drop_claim(key);
                ProviderRunResult::new(TimerCompletion::success(1), ProviderDirective::Stop)
            }
        },
    )?;
    if let Err(error) = registration.ensure_scheduled(TimerSchedule::After(delay)) {
        return Err(rollback_registration(
            TimerClaim::Once(registration),
            error.into(),
        ));
    }
    retain_claim(key, TimerClaim::Once(registration))?;
    Ok(TimerClaimId(key))
}

fn register_transient_result_once(
    scope: &str,
    delay: Duration,
    label: String,
    task: impl Future<Output = TimerRunResult> + 'static,
) -> Result<TimerClaimId, TimerError> {
    require_active()?;
    discard_expired_transient_claims()?;
    let (key, identity) = next_transient_identity(scope, label)?;
    let mut task = Some(task);
    let registration = register_once(
        identity,
        DeclarationLifetime::RemoveWhenStopped,
        move |_context: OnceContext| {
            let task = task.take();
            async move {
                let result = if let Some(task) = task {
                    task.await.into_provider()
                } else {
                    TimerRunResult::invariant_failure().into_provider()
                };
                drop_claim(key);
                ProviderRunResult::new(result.completion(), ProviderDirective::Stop)
            }
        },
    )?;
    if let Err(error) = registration.ensure_scheduled(TimerSchedule::After(delay)) {
        return Err(rollback_registration(
            TimerClaim::Once(registration),
            error.into(),
        ));
    }
    retain_claim(key, TimerClaim::Once(registration))?;
    Ok(TimerClaimId(key))
}

fn next_transient_identity(
    scope: &str,
    label: String,
) -> Result<(ClaimKey, TimerIdentity), TimerError> {
    let id = NEXT_TRANSIENT_ID.with(|next| {
        let id = next
            .get()
            .checked_add(1)
            .ok_or(TimerError::IdentityExhausted)?;
        next.set(id);
        Ok::<_, TimerError>(id)
    })?;
    let identity = TimerIdentity::try_new("canic", format!("{scope}-{id}"), label)?;
    Ok((ClaimKey::Transient(id), identity))
}

fn require_active() -> Result<(), TimerError> {
    if TIMERS_SUSPENDED.with(Cell::get) {
        return Err(TimerError::Suspended);
    }
    Ok(())
}

fn with_builtin(
    key: TimerKey,
    operation: impl FnOnce(&OnceRegistration) -> Result<(), ProviderError>,
) -> Result<(), TimerError> {
    with_claim(ClaimKey::BuiltIn(key), |claim| match claim {
        TimerClaim::Once(registration) => operation(registration).map_err(TimerError::from),
        TimerClaim::AfterCompletion(_) => Err(TimerError::WrongPolicy),
    })?
    .ok_or(TimerError::MissingClaim)?
}

fn with_claim<T>(
    key: ClaimKey,
    operation: impl FnOnce(&TimerClaim) -> T,
) -> Result<Option<T>, TimerError> {
    CLAIMS
        .try_with(|claims| {
            let claims = claims.try_borrow().map_err(|_| TimerError::CustodyBusy)?;
            Ok::<_, TimerError>(claims.get(&key).map(operation))
        })
        .map_err(|_| TimerError::CustodyBusy)?
}

fn retain_claim(key: ClaimKey, claim: TimerClaim) -> Result<(), TimerError> {
    retain_with_rollback(
        claim,
        |claim| {
            CLAIMS.with(|claims| {
                let Ok(mut claims) = claims.try_borrow_mut() else {
                    return Err((TimerError::CustodyBusy, claim));
                };
                if claims.contains_key(&key) {
                    return Err((TimerError::WrongPolicy, claim));
                }
                claims.insert(key, claim);
                Ok(())
            })
        },
        TimerClaim::unregister,
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

fn rollback_registration(claim: TimerClaim, primary: TimerError) -> TimerError {
    match claim.unregister() {
        Ok(()) => primary,
        Err(cleanup) => TimerError::RegistrationRollback {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        },
    }
}

fn remove_claim(key: ClaimKey) -> Result<TimerClaim, TimerError> {
    CLAIMS
        .try_with(|claims| {
            claims
                .try_borrow_mut()
                .map_err(|_| TimerError::CustodyBusy)?
                .remove(&key)
                .ok_or(TimerError::MissingClaim)
        })
        .map_err(|_| TimerError::CustodyBusy)?
}

fn drop_claim(key: ClaimKey) {
    let _ = CLAIMS.try_with(|claims| {
        if let Ok(mut claims) = claims.try_borrow_mut() {
            claims.remove(&key);
        }
    });
}

fn discard_expired_transient_claims() -> Result<(), TimerError> {
    let expired = CLAIMS
        .try_with(|claims| {
            let claims = claims.try_borrow().map_err(|_| TimerError::CustodyBusy)?;
            claims
                .iter()
                .filter(|(key, _)| matches!(key, ClaimKey::Transient(_)))
                .map(|(key, claim)| (*key, claim.identity().clone()))
                .map(|(key, identity)| Ok(timer_snapshot(&identity)?.is_none().then_some(key)))
                .collect::<Result<Vec<_>, TimerError>>()
        })
        .map_err(|_| TimerError::CustodyBusy)??
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if expired.is_empty() {
        return Ok(());
    }
    CLAIMS
        .try_with(|claims| {
            let mut claims = claims
                .try_borrow_mut()
                .map_err(|_| TimerError::CustodyBusy)?;
            for key in expired {
                claims.remove(&key);
            }
            Ok(())
        })
        .map_err(|_| TimerError::CustodyBusy)?
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
    use std::{cell::Cell, collections::BTreeSet};

    #[test]
    fn fixed_claim_identities_are_exact_and_unique() {
        let identities = TimerKey::ALL
            .into_iter()
            .map(|key| key.identity().expect("fixed timer identity"))
            .collect::<BTreeSet<_>>();

        assert_eq!(identities.len(), TimerKey::ALL.len());
        assert!(
            identities
                .iter()
                .all(|identity| identity.owner() == "canic")
        );
        assert!(identities.iter().any(|identity| {
            identity.subsystem() == "intent_cleanup" && identity.name() == "run"
        }));
    }

    #[test]
    fn root_and_nonroot_declarations_match_actual_runtime_owners() {
        assert_eq!(
            TimerKey::ROOT.into_iter().collect::<BTreeSet<_>>(),
            [
                TimerKey::AuthRenewal,
                TimerKey::CycleTopup,
                TimerKey::IntentCleanup,
                TimerKey::LogRetention,
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            TimerKey::NONROOT.into_iter().collect::<BTreeSet<_>>(),
            [
                TimerKey::CycleTopup,
                TimerKey::IntentCleanup,
                TimerKey::LogRetention,
                TimerKey::PlacementReceiptAcknowledgement,
            ]
            .into_iter()
            .collect()
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
    fn built_in_results_preserve_completion_and_directive() {
        let result = TimerRunResult {
            outcome: TimerExecutionOutcome::RetryableFailure,
            work_count: 3,
            directive: TimerDirective::RetryAfter(Duration::from_secs(5)),
        }
        .into_provider();

        assert_eq!(
            result.completion().outcome(),
            ic_timers::TimerCompletionOutcome::RetryableFailure
        );
        assert_eq!(result.completion().work_count(), 3);
        assert_eq!(
            result.directive(),
            ProviderDirective::RetryAfter(Duration::from_secs(5))
        );
    }

    #[test]
    fn snapshot_resume_invokes_the_registered_domain_participant() {
        thread_local! {
            static CALLED: Cell<bool> = const { Cell::new(false) };
        }
        #[expect(
            clippy::unnecessary_wraps,
            reason = "the test double must implement the fallible participant signature"
        )]
        fn participant() -> Result<(), TimerError> {
            CALLED.with(|called| called.set(true));
            Ok(())
        }

        TimerWorkflow::register_snapshot_resume_participant(participant);
        TimerWorkflow::resume_all().expect("resume timer owners");

        assert!(CALLED.with(Cell::get));
        SNAPSHOT_RESUME_PARTICIPANT.with(|current| current.set(None));
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
}
