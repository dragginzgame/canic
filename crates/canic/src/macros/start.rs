// -----------------------------------------------------------------------------
// Start macros
// -----------------------------------------------------------------------------

// Lifecycle core for non-root Canic canisters.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_nonroot_lifecycle_core {
    ($canister_role:expr) => {
        #[::canic::cdk::init]
        fn init(payload: ::canic::dto::abi::v1::CanisterInitPayload, args: Option<Vec<u8>>) {
            let (config_str, config_path) = $crate::__canic_load_config!();

            $crate::__internal::core::api::lifecycle::LifecycleApi::init_nonroot_canister(
                $canister_role,
                payload,
                args.clone(),
                config_str,
                config_path,
            );

            $crate::__canic_start_nonroot_user_timers!(args);
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            let (config_str, config_path) = $crate::__canic_load_config!();

            $crate::__internal::core::api::lifecycle::LifecycleApi::post_upgrade_nonroot_canister(
                $canister_role,
                config_str,
                config_path,
            );

            $crate::__canic_start_nonroot_upgrade_timers!();
        }
    };
}

// Lifecycle core for the root Canic canister.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_root_lifecycle_core {
    () => {
        #[::canic::cdk::init]
        fn init(identity: ::canic::dto::subnet::SubnetIdentity) {
            let (config_str, config_path) = $crate::__canic_load_config!();

            $crate::__internal::core::api::lifecycle::LifecycleApi::init_root_canister(
                identity,
                config_str,
                config_path,
            );

            $crate::__canic_start_root_user_timers!();
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            let (config_str, config_path) = $crate::__canic_load_config!();

            $crate::__internal::core::api::lifecycle::LifecycleApi::post_upgrade_root_canister(
                config_str,
                config_path,
            );

            $crate::__canic_start_root_upgrade_timers!();
        }
    };
}

// User lifecycle timer bundle for non-root init.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_nonroot_user_timers {
    ($args:expr) => {
        $crate::__internal::core::api::timer::TimerApi::set_lifecycle_timer(
            ::std::time::Duration::ZERO,
            "canic:user:init",
            async move {
                canic_setup().await;
                canic_install($args).await;
            },
        );
    };
}

// User lifecycle timer bundle for non-root upgrades.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_nonroot_upgrade_timers {
    () => {
        $crate::__internal::core::api::timer::TimerApi::set_lifecycle_timer(
            ::core::time::Duration::ZERO,
            "canic:user:init",
            async move {
                canic_setup().await;
                canic_upgrade().await;
            },
        );
    };
}

// User lifecycle timer bundle for root init.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_root_user_timers {
    () => {
        $crate::__internal::core::api::timer::TimerApi::set_lifecycle_timer(
            ::core::time::Duration::ZERO,
            "canic:user:init",
            async move {
                canic_setup().await;
                canic_install().await;
            },
        );
    };
}

// User lifecycle timer bundle for root upgrades.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_root_upgrade_timers {
    () => {
        $crate::__internal::core::api::timer::TimerApi::set_lifecycle_timer(
            ::core::time::Duration::ZERO,
            "canic:user:init",
            async move {
                canic_setup().await;
                canic_upgrade().await;
            },
        );
    };
}

// Default non-root capability bundle composition.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_nonroot_capability_bundles {
    () => {
        $crate::canic_endpoints!();
        $crate::canic_endpoints_nonroot!();
    };
}

// Default root capability bundle composition.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_root_capability_bundles {
    () => {
        $crate::canic_endpoints!();
        $crate::canic_endpoints_root!();
    };
}

/// Configure lifecycle hooks for **non-root** Canic canisters.
///
/// This macro defines the IC-required `init` and `post_upgrade` entry points
/// at the crate root and immediately delegates lifecycle semantics to runtime
/// adapters after performing minimal bootstrap.
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
        $crate::__canic_start_nonroot_lifecycle_core!($canister_role);
        $crate::__canic_start_nonroot_capability_bundles!();
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
        $crate::__canic_start_root_lifecycle_core!();
        $crate::__canic_start_root_capability_bundles!();
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
        let config_path = env!("CANIC_CONFIG_PATH");
        let config_str = include_str!(env!("CANIC_CONFIG_PATH"));
        (config_str, config_path)
    }};
}
