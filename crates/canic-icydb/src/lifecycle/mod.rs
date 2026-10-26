//! Host lifecycle mounts the consumer's native actor binding.

/// Include the generated private actor module emitted by the consumer's IcyDB build script.
///
/// The final host must define `icydb_memory_pool() ->
/// Result<icydb::db::MemoryAllocationPool, icydb::db::MemoryAllocationPoolError>`.
/// Cold bootstrap uses this explicit pool's namespace grants and physical
/// exclusions. An already committed host is adopted without invoking the provider.
/// Permanent component keys never choose IDs or grant their own namespace.
/// The generated public `icydb_memory_requests()` factory returns all database
/// requests without opening stores. A composed host combines those requests with
/// other components and passes a checked `icydb::db::SealedDeclarationSnapshot` to
/// Memory bootstrap, invoking `icydb::db::prepare_memory_bootstrap` in its policy.
/// IcyDB then verifies its requirements against that committed host once.
///
/// The zero-argument form owns hidden install and post-upgrade lifecycle
/// entries for canisters without application hooks. Applications that own
/// the complete lifecycle root use participant mode and invoke the matching
/// hidden IcyDB participant synchronously:
///
/// ```ignore
/// canic_icydb::start!(icydb; participant);
///
/// // Inside the application's own init export:
/// fn init() {
///     crate::__icydb_lifecycle_participant::init();
/// }
///
/// // Inside the application's own post-upgrade export:
/// fn post_upgrade() {
///     crate::__icydb_lifecycle_participant::post_upgrade();
/// }
/// ```
///
/// Applications that want IcyDB to own the lifecycle exports while running
/// application callbacks afterward use the composed form:
///
/// ```ignore
/// canic_icydb::start! {
///     icydb;
///     init(args: InitArgs) => application::init;
///     post_upgrade() => application::post_upgrade;
/// }
/// ```
///
/// Supply each composed hook's argument tuple, including `()`, as raw bytes
/// from `canic_icydb::wire::encode_init_arguments` or `canic_icydb::wire::encode_post_upgrade_arguments`.
///
/// IcyDB registers its startup watchdog before invoking either callback. The
/// callback remains application-owned and must observe `startup_state()`
/// before restoring timers, caches, or other database-dependent state.
#[macro_export]
#[expect(
    clippy::crate_in_macro_def,
    reason = "the consumer supplies its actual facade binding"
)]
macro_rules! start {
    ($database:path) => {
        $crate::start!($database; __standalone);
    };
    ($database:path; __standalone) => {
        use $database as __icydb_facade;
        use $crate::__private::cdk as __icydb_host_sdk;
        crate::__icydb_facade::__icydb_start_actor!();
        $crate::__icydb_host_module!();
        $crate::__icydb_start_lifecycle!();
    };
    ($database:path; participant) => {
        use $database as __icydb_facade;
        use $crate::__private::cdk as __icydb_host_sdk;
        crate::__icydb_facade::__icydb_start_actor!();
        $crate::__icydb_host_module!();
        $crate::__icydb_start_participant_lifecycle!();
    };
    ($database:path;
        init($($init_arg:ident : $init_ty:ty),* $(,)?) => $init:path;
        post_upgrade($($upgrade_arg:ident : $upgrade_ty:ty),* $(,)?) => $post_upgrade:path;
    ) => {
        use $database as __icydb_facade;
        use $crate::__private::cdk as __icydb_host_sdk;
        crate::__icydb_facade::__icydb_start_actor!();
        $crate::__icydb_host_module!();
        $crate::__icydb_start_lifecycle! {
            init($($init_arg: $init_ty),*) => $init;
            post_upgrade($($upgrade_arg: $upgrade_ty),*) => $post_upgrade;
        }
    };
}

#[doc(hidden)]
#[macro_export]
#[expect(
    clippy::crate_in_macro_def,
    reason = "participant functions must call the consuming canister's generated actor"
)]
macro_rules! __icydb_start_participant_lifecycle {
    () => {
        #[doc(hidden)]
        #[allow(
            dead_code,
            reason = "downstream canisters may select participant hooks through generated lifecycle wiring"
        )]
        pub(crate) mod __icydb_lifecycle_participant {
            use std::cell::Cell;

            #[derive(Clone, Copy)]
            enum State {
                Idle,
                Running,
                Completed,
            }

            std::thread_local! {
                static STATE: Cell<State> = const { Cell::new(State::Idle) };
            }

            // Native traps unwind instead of rolling back a replicated IC
            // message. Reset only that test/runtime model so retry exercises
            // the same latch transition that message rollback provides on Wasm.
            #[cfg(not(target_family = "wasm"))]
            struct NativeRollbackGuard {
                completed: bool,
            }

            #[cfg(not(target_family = "wasm"))]
            impl NativeRollbackGuard {
                const fn new() -> Self {
                    Self { completed: false }
                }

                const fn complete(&mut self) {
                    self.completed = true;
                }
            }

            #[cfg(not(target_family = "wasm"))]
            impl Drop for NativeRollbackGuard {
                fn drop(&mut self) {
                    if !self.completed {
                        STATE.with(|state| state.set(State::Idle));
                    }
                }
            }

            // Both lifecycle phases deliberately converge here. Completed is
            // shared across them so any duplicate returns before generated
            // timer or recovery state can be observed or changed.
            fn participate(work: fn() -> ()) {
                let should_run = STATE.with(|state| match state.get() {
                    State::Idle => {
                        state.set(State::Running);
                        true
                    }
                    State::Running => $crate::__private::cdk::api::trap(
                        "IcyDB lifecycle participant re-entered while running",
                    ),
                    State::Completed => false,
                });

                if !should_run {
                    return;
                }

                #[cfg(not(target_family = "wasm"))]
                let mut rollback = NativeRollbackGuard::new();

                work();
                STATE.with(|state| state.set(State::Completed));

                #[cfg(not(target_family = "wasm"))]
                rollback.complete();
            }

            /// Participate synchronously in the canister's init lifecycle.
            #[doc(hidden)]
            pub(crate) fn init() -> () {
                let participate: fn() -> () = crate::__icydb_host::__icydb_startup_init;
                self::participate(participate);
            }

            /// Participate synchronously in the canister's post-upgrade lifecycle.
            #[doc(hidden)]
            pub(crate) fn post_upgrade() -> () {
                let participate: fn() -> () =
                    crate::__icydb_host::__icydb_startup_post_upgrade;
                self::participate(participate);
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
#[expect(
    clippy::crate_in_macro_def,
    reason = "lifecycle wrappers must call the consuming canister's generated actor"
)]
macro_rules! __icydb_start_lifecycle {
    () => {
        #[$crate::__private::native_entry(init, name = "", sdk = crate::__icydb_host_sdk)]
        fn __icydb_startup_init() {
            crate::__icydb_host::__icydb_startup_init();
        }

        #[$crate::__private::native_entry(post_upgrade, name = "", sdk = crate::__icydb_host_sdk)]
        fn __icydb_startup_post_upgrade() {
            crate::__icydb_host::__icydb_startup_post_upgrade();
        }
    };

    (
        init($($init_arg:ident : $init_ty:ty),* $(,)?) => $init:path;
        post_upgrade($($upgrade_arg:ident : $upgrade_ty:ty),* $(,)?) => $post_upgrade:path;
    ) => {
        #[$crate::__private::native_entry(init, name = "", sdk = crate::__icydb_host_sdk)]
        fn __icydb_startup_init() {
            let ($($init_arg,)*): ($($init_ty,)*) = match $crate::read_arguments(crate::__icydb_facade::wire::MAX_REQUEST_BYTES).ok_or(crate::__icydb_facade::wire::Error::Limit).and_then(|bytes| $crate::wire::decode_init_arguments(&bytes, crate::__icydb_facade::wire::limits(crate::__icydb_facade::wire::MAX_REQUEST_BYTES))) {
                Ok(arguments) => arguments,
                Err(_) => $crate::__private::cdk::api::trap("invalid IcyDB init arguments"),
            };
            crate::__icydb_host::__icydb_startup_init();
            let (): () = crate::__icydb_facade::db::with_request_execution(|| ($init)($($init_arg),*));
        }

        #[$crate::__private::native_entry(post_upgrade, name = "", sdk = crate::__icydb_host_sdk)]
        fn __icydb_startup_post_upgrade() {
            let ($($upgrade_arg,)*): ($($upgrade_ty,)*) = match $crate::read_arguments(crate::__icydb_facade::wire::MAX_REQUEST_BYTES).ok_or(crate::__icydb_facade::wire::Error::Limit).and_then(|bytes| $crate::wire::decode_post_upgrade_arguments(&bytes, crate::__icydb_facade::wire::limits(crate::__icydb_facade::wire::MAX_REQUEST_BYTES))) {
                Ok(arguments) => arguments,
                Err(_) => $crate::__private::cdk::api::trap("invalid IcyDB post-upgrade arguments"),
            };
            crate::__icydb_host::__icydb_startup_post_upgrade();
            let (): () = crate::__icydb_facade::db::with_request_execution(
                || ($post_upgrade)($($upgrade_arg),*)
            );
        }
    };
}
