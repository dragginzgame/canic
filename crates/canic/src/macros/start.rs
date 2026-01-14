// -----------------------------------------------------------------------------
// Start macros
// -----------------------------------------------------------------------------

/// Configure lifecycle hooks for **non-root** Canic canisters.
///
/// This macro defines the IC-required `init` and `post_upgrade` entry points
/// at the crate root and immediately delegates lifecycle semantics to runtime
/// adapters after performing minimal bootstrap
///
/// IMPORTANT:
/// - This macro must remain **thin**
/// - It must not perform orchestration
/// - It must not perform async work inline
/// - It must not encode policy
/// - It may schedule async hooks via timers, but must never await them
///
/// Its sole responsibility is to bridge IC lifecycle hooks to runtime code.
#[macro_export]
macro_rules! start {
    ($canister_role:expr) => {
        #[::canic::cdk::init]
        fn init(payload: ::canic::dto::abi::v1::CanisterInitPayload, args: Option<Vec<u8>>) {
            // Load embedded configuration early.
            $crate::__canic_load_config!();

            // Delegate to lifecycle adapter (NOT workflow).
            $crate::__internal::core::api::lifecycle::LifecycleApi::init_nonroot_canister(
                $canister_role,
                payload,
                args.clone(),
            );

            // ---- userland lifecycle hooks (scheduled last) ----
            $crate::__internal::core::api::timer::TimerApi::set_lifecycle_timer(
                ::std::time::Duration::ZERO,
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
            $crate::__canic_load_config!();

            // Delegate to lifecycle adapter.
            $crate::__internal::core::api::lifecycle::LifecycleApi::post_upgrade_nonroot_canister(
                $canister_role,
            );

            // ---- userland lifecycle hooks (scheduled last) ----
            $crate::__internal::core::api::timer::TimerApi::set_lifecycle_timer(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_upgrade().await;
                },
            );
        }

        $crate::canic_endpoints!();
        $crate::canic_endpoints_nonroot!();
    };
}

/// Configure lifecycle hooks for the **root orchestrator** canister.
///
/// This macro behaves like [`start!`], but delegates to root-specific
/// lifecycle adapters.
///
/// IMPORTANT:
/// - The macro does NOT perform root orchestration
/// - The macro does NOT import WASMs
/// - The macro does NOT create canisters
/// - The macro may schedule async hooks via timers, but must never await them
///
#[macro_export]
macro_rules! start_root {
    () => {
        #[::canic::cdk::init]
        fn init(identity: ::canic::dto::subnet::SubnetIdentity) {
            // Load embedded configuration early.
            $crate::__canic_load_config!();

            // Delegate to lifecycle adapter.
            $crate::__internal::core::api::lifecycle::LifecycleApi::init_root_canister(identity);

            // ---- userland lifecycle hooks (scheduled last) ----
            $crate::__internal::core::api::timer::TimerApi::set_lifecycle_timer(
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
            $crate::__canic_load_config!();

            // Delegate to lifecycle adapter.
            $crate::__internal::core::api::lifecycle::LifecycleApi::post_upgrade_root_canister();

            // ---- userland lifecycle hooks (scheduled last) ----
            $crate::__internal::core::api::timer::TimerApi::set_lifecycle_timer(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_upgrade().await;
                },
            );
        }

        $crate::canic_endpoints!();
        $crate::canic_endpoints_root!();
    };
}

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
        if let Err(err) = $crate::__internal::core::bootstrap::init_config(config_str) {
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
