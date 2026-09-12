//! Module: workflow::fixture_provisioning::timer
//!
//! Responsibility: dispatch bounded imports from a pre-armed native watchdog.
//! Does not own: application rows/cursors, grants, funding or provider bookkeeping.
//! Boundary: the watchdog survives callback traps; durable attempts fence source replies.

use crate::{
    InternalError,
    dto::fixture_provisioning::{FixtureImportFailure, FixtureProvisioningStatus},
    ops::{
        fixture_importer,
        ic::IcOps,
        runtime::{env::EnvOps, fleet_activation::FleetActivationRuntimeOps},
        storage::async_job_recovery::{
            AsyncJobAttempt, AsyncJobCompletion, AsyncJobOwner, AsyncJobRecoveryOps,
        },
    },
    workflow::runtime::{
        async_job::AsyncJobWorkflow,
        timer::{RECOVERY_WATCHDOG_CADENCE, TimerError, require_active},
    },
};
use ic_timers::{
    TimerCadence, TimerCompletion, TimerIdentity, WatchdogDecision, WatchdogReconcileState,
    WatchdogRegistration, WatchdogRunResult, reconcile_watchdog,
};
use std::cell::RefCell;

thread_local! {
    static TIMER: RefCell<Option<WatchdogRegistration>> = const { RefCell::new(None) };
}

/// Canic-owned delivery; durable demand remains the installed assignment and receipt.
pub struct FixtureImportTimer;

impl FixtureImportTimer {
    pub(crate) fn timer_identity() -> Result<TimerIdentity, TimerError> {
        TimerIdentity::try_new("canic", "fixture_import", "deliver").map_err(Into::into)
    }

    /// Lifecycle may precede registration; defer every application callback.
    pub fn start() -> Result<(), InternalError> {
        if EnvOps::is_root()
            || EnvOps::canister_role()?.is_wasm_store()
            || FleetActivationRuntimeOps::is_standalone_local()
        {
            return Ok(());
        }
        if fixture_importer::assignment()
            .map_err(|_| InternalError::invariant())?
            .is_none()
            || fixture_importer::require_active().is_err()
            || AsyncJobRecoveryOps::fixture_import_failure().is_some()
        {
            return Ok(());
        }
        Self::reconcile(WatchdogReconcileState::ScheduledImmediately)?;
        Ok(())
    }

    fn reconcile(desired: WatchdogReconcileState) -> Result<(), TimerError> {
        require_active()?;
        let identity = Self::timer_identity()?;
        let cadence = TimerCadence::new(RECOVERY_WATCHDOG_CADENCE)?;
        TIMER.with(|owner| {
            let mut owner = owner
                .try_borrow_mut()
                .map_err(|_| TimerError::CustodyBusy)?;
            reconcile_watchdog(&mut owner, &identity, cadence, desired, |_context| {
                Self::dispatch()
            })
            .map_err(Into::into)
        })
    }

    /// The provider pre-arms this work's successor in a separate IC message.
    fn dispatch() -> WatchdogRunResult {
        let now = IcOps::now_nanos();
        if AsyncJobRecoveryOps::fixture_import_failure().is_some() {
            return WatchdogRunResult::new(TimerCompletion::no_work(), WatchdogDecision::Stop);
        }
        if fixture_importer::require_active().is_err() {
            return idle();
        }
        let owner = AsyncJobOwner::FixtureImport;
        if let Some(deadline) = AsyncJobRecoveryOps::active_lease_deadline(owner) {
            if deadline > now {
                return idle();
            }
            fixture_importer::abandon_expired_fetch();
            AsyncJobWorkflow::abandon_expired(owner, now);
        }
        let Ok(attempt) = AsyncJobWorkflow::claim(owner) else {
            return idle();
        };
        ic_cdk::futures::spawn(async move {
            Self::run(attempt).await;
        });
        WatchdogRunResult::new(TimerCompletion::success(1), WatchdogDecision::Continue)
    }

    async fn run(attempt: AsyncJobAttempt) {
        let outcome = super::advance_owned(attempt).await;
        if !AsyncJobRecoveryOps::is_current(attempt) {
            return;
        }
        let (completion, desired) = match outcome {
            Ok(FixtureProvisioningStatus::NotRequired | FixtureProvisioningStatus::Complete(_)) => {
                (
                    AsyncJobCompletion::Success,
                    WatchdogReconcileState::Inactive,
                )
            }
            Ok(FixtureProvisioningStatus::Failed(failure)) => {
                stop_failure(attempt, failure);
                return;
            }
            Ok(FixtureProvisioningStatus::Pending(_)) => (
                AsyncJobCompletion::Success,
                WatchdogReconcileState::ScheduledImmediately,
            ),
            Ok(FixtureProvisioningStatus::AwaitingImporter) => retry(),
            Err(error) => match fixture_importer::permanent_failure(error) {
                Some(failure) => {
                    stop_failure(attempt, failure);
                    return;
                }
                None => retry(),
            },
        };
        match AsyncJobRecoveryOps::finish(attempt, completion, IcOps::now_nanos()) {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => IcOps::trap(format!("fixture completion failed: {error}")),
        }
        Self::reconcile(desired)
            .unwrap_or_else(|error| IcOps::trap(format!("fixture scheduling failed: {error}")));
    }
}

const fn retry() -> (AsyncJobCompletion, WatchdogReconcileState) {
    (
        AsyncJobCompletion::RetryableFailure,
        WatchdogReconcileState::Scheduled,
    )
}

fn stop_failure(attempt: AsyncJobAttempt, failure: FixtureImportFailure) {
    if !AsyncJobRecoveryOps::fail_fixture_import(attempt, failure) {
        return;
    }
    crate::log!(
        crate::log::Topic::Init,
        Warn,
        "fixture import requires review: {failure:?}"
    );
    FixtureImportTimer::reconcile(WatchdogReconcileState::Inactive)
        .unwrap_or_else(|error| IcOps::trap(format!("fixture scheduling failed: {error}")));
}

const fn idle() -> WatchdogRunResult {
    WatchdogRunResult::new(TimerCompletion::no_work(), WatchdogDecision::Continue)
}
