//! Lifecycle-owned startup scheduling over the database's bounded native driver.

use ic_timers::{
    TimerCadence, TimerIdentity, WatchdogReconcileState, WatchdogRegistration, WatchdogRunResult,
};
use std::{cell::RefCell, time::Duration};

/// Native database startup demand; scheduler policy stays in this adapter owner.
pub enum StartupProgress {
    /// The bounded driver finished all current work.
    Complete,
    /// Another bounded step can proceed immediately.
    MoreWork,
    /// An asynchronous startup dependency has not completed.
    PendingDependency,
    /// The current attempt failed and can retry at the normal cadence.
    RetryableFailure,
    /// The durable database state refuses further startup attempts.
    InvalidState,
}

thread_local! {
    static STARTUP_WATCHDOG: RefCell<Option<WatchdogRegistration>> = const { RefCell::new(None) };
}

/// Reconcile the adapter's one startup registration from native database demand.
///
/// # Panics
/// Traps on invalid authored identity, reentrant custody or provider failure.
pub fn reconcile_startup_watchdog(active: bool, callback: fn() -> StartupProgress) {
    ic_timers::initialize_runtime().expect("IcyDB timer runtime initialization");
    let identity =
        TimerIdentity::try_new("icydb", "startup", "recovery").expect("startup identity");
    let cadence = TimerCadence::new(Duration::from_secs(1)).expect("startup cadence");
    let desired = if active {
        WatchdogReconcileState::ScheduledImmediately
    } else {
        WatchdogReconcileState::Inactive
    };
    STARTUP_WATCHDOG.with(|slot| {
        let mut registration = slot.try_borrow_mut().expect("startup watchdog custody");
        ic_timers::reconcile_watchdog(&mut registration, &identity, cadence, desired, move |_| {
            match callback() {
                StartupProgress::Complete => WatchdogRunResult::new(
                    ic_timers::TimerCompletion::success(1),
                    ic_timers::WatchdogDecision::Stop,
                ),
                StartupProgress::MoreWork => WatchdogRunResult::new(
                    ic_timers::TimerCompletion::success(1),
                    ic_timers::WatchdogDecision::ContinueImmediately,
                ),
                StartupProgress::PendingDependency => WatchdogRunResult::new(
                    ic_timers::TimerCompletion::no_work(),
                    ic_timers::WatchdogDecision::Continue,
                ),
                StartupProgress::RetryableFailure => WatchdogRunResult::new(
                    ic_timers::TimerCompletion::retryable_failure(0),
                    ic_timers::WatchdogDecision::Continue,
                ),
                StartupProgress::InvalidState => WatchdogRunResult::new(
                    ic_timers::TimerCompletion::invariant_failure(0),
                    ic_timers::WatchdogDecision::Stop,
                ),
            }
        })
        .expect("startup watchdog reconciliation");
    });
}

#[doc(hidden)]
#[macro_export]
#[expect(
    clippy::crate_in_macro_def,
    reason = "host wiring references the consuming native database"
)]
macro_rules! __icydb_host_module {
    () => {
        #[allow(dead_code, reason = "native tests may drive the database without IC timers")]
        mod __icydb_host {
            use crate::__icydb_generated::startup_state;
        fn reconcile_startup_watchdog_from_durable_state(
        ) -> ::std::result::Result<(), crate::__icydb_facade::db::StartupFailure> {
            match startup_state() {
                Ok(
                    crate::__icydb_facade::db::DatabaseStartupState::Ready
                    | crate::__icydb_facade::db::DatabaseStartupState::Recovering,
                ) => {
                    // Ready cardinality construction is optional database work,
                    // but it shares this one lifecycle-owned watchdog.
                    ensure_startup_watchdog_scheduled_immediately();
                    Ok(())
                }
                Err(failure) => {
                    $crate::__private::reconcile_startup_watchdog(false, startup_watchdog_callback);
                    Err(failure)
                }
            }
        }

        pub(crate) fn ensure_startup_watchdog_scheduled_immediately() {
            $crate::__private::reconcile_startup_watchdog(true, startup_watchdog_callback);
        }

        fn register_startup_watchdog_for_lifecycle() {
            if let Err(failure) = reconcile_startup_watchdog_from_durable_state() {
                $crate::__private::cdk::println!(
                    "IcyDB startup failure observed during watchdog reconciliation (E{})",
                    failure.error().code().raw(),
                );
            }
        }

        /// Register generated startup driving before an application install hook.
        #[doc(hidden)]
        pub(crate) fn __icydb_startup_init() {
            register_startup_watchdog_for_lifecycle();
        }

        /// Reconstruct volatile startup driving before an application upgrade hook.
        #[doc(hidden)]
        pub(crate) fn __icydb_startup_post_upgrade() {
            register_startup_watchdog_for_lifecycle();
        }

        fn startup_watchdog_callback() -> $crate::__private::StartupProgress {
            use $crate::__private::StartupProgress;
            $crate::request_boot_entropy(crate::__icydb_facade::db::provide_boot_entropy);
            if startup_state().is_err() { return StartupProgress::InvalidState; }
            match crate::__icydb_facade::db::with_request_execution(crate::__icydb_generated::startup_driver_attempt) {
                Ok(true) => StartupProgress::Complete,
                Ok(false) => StartupProgress::MoreWork,
                Err(error) if error.code() == crate::__icydb_facade::ErrorCode::RUNTIME_BOUNDARY_DATABASE_STARTUP_RECOVERY_PENDING => StartupProgress::PendingDependency,
                Err(error) => {
                    $crate::__private::cdk::println!("IcyDB startup driver retryable failure (E{})", error.code().raw());
                    StartupProgress::RetryableFailure
                }
            }
        }
        }
    };
}
