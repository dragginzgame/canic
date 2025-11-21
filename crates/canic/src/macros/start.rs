//! # Canic Lifecycle Macros
//!
//! These macros define **compile-time lifecycle entry points** (`init` and `post_upgrade`)
//! for Canic canisters. Lifecycle hooks must exist at the crate root with fixed names,
//! so they cannot be registered dynamically — macros are therefore used to generate the
//! boilerplate pre- and post-initialization logic automatically.
//!
//! Each macro sets up configuration, memory, timers, and TLS before calling user-defined
//! async setup functions (`canic_setup`, `canic_install`, `canic_upgrade`), and then
//! exposes the standard Canic endpoint suites.
//!
//! ## When to use which
//!
//! - [`macro@canic_start`] — for **non-root** canisters (standard services, workers, etc.).
//! - [`macro@canic_start_root`] — for the **root orchestrator**, which performs
//!   additional initialization for global registries and root-only extensions.

/// Configure lifecycle hooks for **non-root Canic canisters**.
///
/// This macro wires up the `init` and `post_upgrade` entry points required by the IC,
/// performing pre-initialization steps (config, memory, TLS, environment) before invoking
/// user async functions:
///
/// ```ignore
/// async fn canic_setup() { /* shared setup */ }
/// async fn canic_install(args: Option<Vec<u8>>) { /* called after init */ }
/// async fn canic_upgrade() { /* called after post_upgrade */ }
/// ```
///
/// These functions are spawned asynchronously after bootstrap completes.
/// The macro also exposes the standard non-root Canic endpoint suites.
///
/// This macro must be used instead of a normal function because the IC runtime requires
/// `init` and `post_upgrade` to be declared at the top level.

#[macro_export]
macro_rules! canic_start {
    ($canister_type:expr) => {
        #[::canic::cdk::init]
        fn init(payload: ::canic::ops::CanisterInitPayload, args: Option<Vec<u8>>) {
            ::canic::__canic_load_config!();

            // ops
            ::canic::ops::lifecycle::nonroot_init($canister_type, payload);

            // timers — async body, no spawn()
            let _ =
                ::canic::cdk::timers::set_timer(::std::time::Duration::from_secs(0), async move {
                    canic_setup().await;
                    canic_install(args).await;
                });
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            ::canic::__canic_load_config!();

            // ops
            ::canic::ops::lifecycle::nonroot_post_upgrade($canister_type);

            // timers — async body, no spawn()
            let _ =
                ::canic::cdk::timers::set_timer(::std::time::Duration::from_secs(0), async move {
                    canic_setup().await;
                    canic_upgrade().await;
                });
        }

        ::canic::canic_endpoints!();
        ::canic::canic_endpoints_nonroot!();
    };
}

///
/// Configure lifecycle hooks for the **root Canic orchestrator canister**.
///
/// This macro behaves like [`macro@canic_start`], but includes additional
/// root-only initialization for:
///
/// - the global subnet registry
/// - root-only memory extensions and cycle tracking
/// - the root endpoint suite
///
/// It generates the `init` and `post_upgrade` hooks required by the IC, loads embedded
/// configuration, imports the root `WASMS` bundle, and runs pre- and post-upgrade logic
/// in [`ops::lifecycle`].
///
/// Use this for the root orchestrator canister only. Other canisters should use
/// [`macro@canic_start`].

#[macro_export]
macro_rules! canic_start_root {
    () => {
        #[::canic::cdk::init]
        fn init(identity: ::canic::memory::topology::SubnetIdentity) {
            ::canic::__canic_load_config!();

            // ops
            ::canic::ops::lifecycle::root_init(identity);

            // import wasms
            ::canic::state::wasm::WasmRegistry::import(WASMS);

            // timers
            let _ =
                ::canic::cdk::timers::set_timer(std::time::Duration::from_secs(0), async move {
                    ::canic::ops::root::root_set_subnet_id().await;
                    ::canic::ops::root::root_create_canisters().await.unwrap();

                    canic_setup().await;
                    canic_install().await;
                });
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            ::canic::__canic_load_config!();
            ::canic::state::wasm::WasmRegistry::import(WASMS);

            // ops
            ::canic::ops::lifecycle::root_post_upgrade();

            // timers
            let _ =
                ::canic::cdk::timers::set_timer(::std::time::Duration::from_secs(0), async move {
                    canic_setup().await;
                    canic_upgrade().await;
                });
        }

        ::canic::canic_endpoints!();
        ::canic::canic_endpoints_root!();
    };
}

//
// Private helpers
//

///
/// Load the embedded configuration during init and upgrade hooks.
///
/// This macro exists solely to embed and load the TOML configuration file
/// at compile time (`CANIC_CONFIG_PATH`). It is used internally by
/// [`macro@canic_start`] and [`macro@canic_start_root`].

#[doc(hidden)]
#[macro_export]
macro_rules! __canic_load_config {
    () => {
        #[cfg(canic)]
        {
            let config_str = include_str!(env!("CANIC_CONFIG_PATH"));
            $crate::config::Config::init_from_toml(config_str).unwrap();
        }
    };
}
