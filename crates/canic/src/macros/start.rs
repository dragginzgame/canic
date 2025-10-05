//! Macros used to bootstrap Canic canisters.

/// Configure lifecycle hooks for non-root Canic canisters.
///
/// This macro wires up `init` and `post_upgrade` entry points that bootstrap
/// configuration, memory, timers, and eager TLS state before deferring to the
/// user-provided `canic_setup`, `canic_install`, and `canic_upgrade`
/// functions. It also exposes the standard Canic endpoint suites.
#[macro_export]
macro_rules! canic_start {
    ($canister_type:expr) => {
        #[::canic::cdk::init]
        fn init(
            state: ::canic::memory::state::CanisterStateData,
            parents: Vec<::canic::memory::CanisterSummary>,
            args: Option<Vec<u8>>,
        ) {
            // log (generic, no state info yet)
            ::canic::log!(::canic::Log::Info, "🏁 init: {}", $canister_type);

            // config
            ::canic::__canic_load_config!();

            // tls
            ::canic::eager::init_eager_tls(); // ⚠️ MUST precede init_memory

            // memory
            ::canic::memory::registry::init_memory();
            ::canic::memory::context::CanisterContext::set_root_pid(::canic::cdk::api::msg_caller());
            ::canic::memory::state::CanisterState::import(state);
            ::canic::memory::topology::SubnetParents::import(parents);

            // CYCLES
            ::canic::memory::capability::cycles::CycleTracker::start();

            // timers
            let _ = ::canic::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::canic::cdk::futures::spawn(async move {
                    canic_setup().await;
                    canic_install(args).await;
                });
            });
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            // log
            ::canic::log!(::canic::Log::Info, "🏁 post_upgrade: {}", $canister_type);

            // config
            ::canic::__canic_load_config!();

            // tls
            ::canic::eager::init_eager_tls(); // ⚠️ MUST precede init_memory

            // cycles
            ::canic::memory::capability::cycles::CycleTracker::start();

            // timers
            let _ = ::canic::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                ::canic::cdk::futures::spawn(async move {
                    canic_setup().await;
                    canic_upgrade().await;
                });
            });
        }

        ::canic::canic_endpoints!();
        ::canic::canic_endpoints_nonroot!();
    };
}

/// Configure lifecycle hooks for the root Canic orchestrator canister.
///
/// Similar to [`macro@canic_start`] but tailored to the root environment: it
/// initializes the global subnet registry, root-only capabilities, and
/// exports root-specific endpoints.
#[macro_export]
macro_rules! canic_start_root {
    () => {
        #[::canic::cdk::init]
        fn init() {
            // log
            ::canic::cdk::println!("");
            ::canic::log!(
                ::canic::Log::Info,
                "------------------------------------------------------------"
            );
            ::canic::log!(::canic::Log::Info, "🏁 init: root");

            // config
            ::canic::__canic_load_config!();

            // tls
            ::canic::eager::init_eager_tls();

            // memory
            ::canic::memory::registry::init_memory();
            let entry = ::canic::memory::topology::SubnetTopology::init_root(
                ::canic::cdk::api::canister_self(),
            );
            ::canic::memory::context::CanisterContext::set_root_pid(
                ::canic::cdk::api::canister_self(),
            );
            ::canic::memory::state::CanisterState::set_canister(entry.into());

            // state
            ::canic::state::wasm::WasmRegistry::import(WASMS);

            // cycles
            ::canic::memory::capability::cycles::CycleTracker::start();

            // root only
            ::canic::memory::root::CanisterReserve::start();

            // timers
            let _ =
                ::canic::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                    ::canic::cdk::futures::spawn(async move {
                        //
                        // GET SUBNET
                        //
                        //     let res = ::canic::interface::ic::get_current_subnet().await;
                        //     panic!("{res:?}");

                        canic_setup().await;
                        canic_install().await;
                    });
                });
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            // log
            ::canic::log!(::canic::Log::Info, "🏁 post_upgrade: root");

            // config
            ::canic::__canic_load_config!();

            // tls
            ::canic::eager::init_eager_tls();

            // state
            ::canic::state::wasm::WasmRegistry::import(WASMS);

            // cycles
            ::canic::memory::capability::cycles::CycleTracker::start();

            // root only
            ::canic::memory::root::CanisterReserve::start();

            // timers
            let _ =
                ::canic::cdk::timers::set_timer(::std::time::Duration::from_secs(0), move || {
                    ::canic::cdk::futures::spawn(async move {
                        canic_setup().await;
                        canic_upgrade().await;
                    });
                });
        }

        ::canic::canic_endpoints!();
        ::canic::canic_endpoints_root!();
    };
}

//
// Private helpers
//

/// Load the embedded configuration during init and upgrade hooks.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_load_config {
    () => {
        #[cfg(any(canic))]
        {
            let config_str = include_str!(env!("CANIC_CONFIG_PATH"));
            $crate::expect_or_trap(
                $crate::config::Config::init_from_toml(config_str),
                "init config",
            )
        }
    };
}
