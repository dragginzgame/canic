/// Configure lifecycle hooks for **non-root** Canic canisters.
///
/// This macro defines the IC-required `init` and `post_upgrade` entry points
/// at the crate root and *immediately delegates* all real work to runtime
/// bootstrap code.
///
/// IMPORTANT:
/// - This macro must remain **thin**
/// - It must not schedule timers
/// - It must not perform orchestration
/// - It must not contain async logic
/// - It must not encode policy
///
/// Its sole responsibility is to bridge IC lifecycle hooks to runtime code.
#[macro_export]
macro_rules! start {
    ($canister_role:expr) => {
        #[::canic::cdk::init]
        fn init(payload: ::canic::core::abi::CanisterInitPayload, args: Option<Vec<u8>>) {
            // Load embedded configuration early.
            ::canic::core::__canic_load_config!();

            // Delegate to lifecycle adapter (NOT workflow).
            ::canic::core::lifecycle::init::nonroot_init($canister_role, payload, args.clone());

            // ---- userland lifecycle hooks (scheduled last) ----
            ::canic::core::ops::ic::timer::TimerOps::set(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_install(args).await;
                },
            );
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            // Reload embedded configuration on upgrade.
            ::canic::core::__canic_load_config!();

            // Delegate to lifecycle adapter.
            ::canic::core::lifecycle::upgrade::nonroot_post_upgrade($canister_role);

            // ---- userland lifecycle hooks (scheduled last) ----
            ::canic::core::ops::ic::timer::TimerOps::set(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_upgrade().await;
                },
            );
        }

        ::canic::core::canic_endpoints!();
        ::canic::core::canic_endpoints_nonroot!();
    };
}

/// Configure lifecycle hooks for the **root orchestrator** canister.
///
/// This macro behaves like [`start!`], but delegates to root-specific
/// bootstrap logic.
///
/// IMPORTANT:
/// - The macro does NOT perform root orchestration
/// - The macro does NOT import WASMs
/// - The macro does NOT create canisters
/// - The macro does NOT schedule timers
///
/// All root-specific behavior lives in `workflow::bootstrap`.
#[macro_export]
macro_rules! start_root {
    () => {
        #[::canic::cdk::init]
        fn init(identity: ::canic::core::dto::registry::SubnetIdentity) {
            // Load embedded configuration early.
            ::canic::core::__canic_load_config!();

            // Delegate to lifecycle adapter.
            ::canic::core::lifecycle::init::root_init(identity);

            // ---- userland lifecycle hooks (scheduled last) ----
            ::canic::core::ops::ic::timer::TimerOps::set(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_install().await;
                },
            );
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            // Reload embedded configuration on upgrade.
            ::canic::core::__canic_load_config!();

            // Delegate to lifecycle adapter.
            ::canic::core::lifecycle::upgrade::root_post_upgrade();

            // ---- userland lifecycle hooks (scheduled last) ----
            ::canic::core::ops::ic::timer::TimerOps::set(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_upgrade().await;
                },
            );
        }

        ::canic::core::canic_endpoints!();
        ::canic::core::canic_endpoints_root!();
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
/// [`macro@canic::start`] and [`macro@canic::start_root`].

#[doc(hidden)]
#[macro_export]
macro_rules! __canic_load_config {
    () => {{
        let config_str = include_str!(env!("CANIC_CONFIG_PATH"));
        if let Err(err) = $crate::config::Config::init_from_toml(config_str) {
            $crate::cdk::println!(
                "[canic] FATAL: config init failed (CANIC_CONFIG_PATH={}): {err}",
                env!("CANIC_CONFIG_PATH")
            );
            let msg = format!(
                "canic init failed: config init failed (CANIC_CONFIG_PATH={}): {err}",
                env!("CANIC_CONFIG_PATH")
            );
            $crate::cdk::api::trap(&msg);
        }
    }};
}
