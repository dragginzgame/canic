//! Lifecycle-owned startup scheduling over the database's bounded native driver.

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
        thread_local! {
            static STARTUP_WATCHDOG_REGISTRATION: ::std::cell::RefCell<
                ::std::option::Option<
                    $crate::__private::ic_timers::WatchdogRegistration
                >
            > = const { ::std::cell::RefCell::new(None) };
        }

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
                    reconcile_startup_watchdog(
                        $crate::__private::ic_timers::WatchdogReconcileState::Inactive,
                    );
                    Err(failure)
                }
            }
        }

        pub(crate) fn ensure_startup_watchdog_scheduled_immediately() {
            reconcile_startup_watchdog(
                $crate::__private::ic_timers::WatchdogReconcileState::ScheduledImmediately,
            );
        }

        fn initialize_startup_timer_runtime() {
            if $crate::__private::ic_timers::initialize_runtime().is_err() {
                $crate::__private::cdk::api::trap(
                    "IcyDB timer runtime initialization failed",
                );
            }
        }

        fn startup_watchdog_identity(
        ) -> $crate::__private::ic_timers::TimerIdentity {
            match $crate::__private::ic_timers::TimerIdentity::try_new(
                "icydb",
                "startup",
                "recovery",
            ) {
                Ok(identity) => identity,
                Err(_) => $crate::__private::cdk::api::trap(
                    "IcyDB startup watchdog identity is invalid",
                ),
            }
        }

        fn startup_watchdog_cadence(
        ) -> $crate::__private::ic_timers::TimerCadence {
            match $crate::__private::ic_timers::TimerCadence::new(
                ::std::time::Duration::from_secs(1),
            ) {
                Ok(cadence) => cadence,
                Err(_) => $crate::__private::cdk::api::trap(
                    "IcyDB startup watchdog cadence is invalid",
                ),
            }
        }

        fn reconcile_startup_watchdog(
            desired: $crate::__private::ic_timers::WatchdogReconcileState,
        ) {
            initialize_startup_timer_runtime();
            let identity = startup_watchdog_identity();
            let cadence = startup_watchdog_cadence();
            STARTUP_WATCHDOG_REGISTRATION.with(|slot| {
                let Ok(mut registration) = slot.try_borrow_mut() else {
                    $crate::__private::cdk::api::trap(
                        "IcyDB startup watchdog reconciliation is reentrant",
                    );
                };
                if $crate::__private::ic_timers::reconcile_watchdog(
                    &mut registration,
                    &identity,
                    cadence,
                    desired,
                    startup_watchdog_callback,
                )
                .is_err()
                {
                    $crate::__private::cdk::api::trap(
                        "IcyDB startup watchdog reconciliation failed",
                    );
                }
            });
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

        fn startup_watchdog_callback(
            _context: $crate::__private::ic_timers::WatchdogContext,
        ) -> $crate::__private::ic_timers::WatchdogRunResult {
            $crate::request_boot_entropy(crate::__icydb_facade::db::provide_boot_entropy);
            match startup_state() {
                Err(_) => {
                    return $crate::__private::ic_timers::WatchdogRunResult::new(
                        $crate::__private::ic_timers::TimerCompletion::invariant_failure(0),
                        $crate::__private::ic_timers::WatchdogDecision::Stop,
                    );
                }
                Ok(
                    crate::__icydb_facade::db::DatabaseStartupState::Ready
                    | crate::__icydb_facade::db::DatabaseStartupState::Recovering,
                ) => {}
            }

            let result = crate::__icydb_facade::db::with_request_execution(crate::__icydb_generated::startup_driver_attempt);
            match result {
                Ok(true) => $crate::__private::ic_timers::WatchdogRunResult::new(
                    $crate::__private::ic_timers::TimerCompletion::success(1),
                    $crate::__private::ic_timers::WatchdogDecision::Stop,
                ),
                Ok(false) => $crate::__private::ic_timers::WatchdogRunResult::new(
                    $crate::__private::ic_timers::TimerCompletion::success(1),
                    $crate::__private::ic_timers::WatchdogDecision::ContinueImmediately,
                ),
                // Asynchronous startup dependencies can be pending on a healthy
                // fresh install or upgrade. Wait at the normal cadence without
                // reporting a failure or spinning immediate successor callbacks.
                Err(error) if error.code()
                    == crate::__icydb_facade::ErrorCode::RUNTIME_BOUNDARY_DATABASE_STARTUP_RECOVERY_PENDING =>
                {
                    $crate::__private::ic_timers::WatchdogRunResult::new(
                        $crate::__private::ic_timers::TimerCompletion::no_work(),
                        $crate::__private::ic_timers::WatchdogDecision::Continue,
                    )
                }
                Err(error) => {
                    $crate::__private::cdk::println!(
                        "IcyDB startup driver retryable failure (E{})",
                        error.code().raw(),
                    );
                    $crate::__private::ic_timers::WatchdogRunResult::new(
                        $crate::__private::ic_timers::TimerCompletion::retryable_failure(0),
                        $crate::__private::ic_timers::WatchdogDecision::Continue,
                    )
                }
            }
        }

        }
    };
}
