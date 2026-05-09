// -----------------------------------------------------------------------------
// Start macros
// -----------------------------------------------------------------------------

// Lifecycle core for non-root Canic canisters.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_nonroot_lifecycle_core {
    ($canister_role:expr $(, $init:block)?) => {
        #[doc(hidden)]
        fn __canic_compiled_config() -> (
            $crate::__internal::core::bootstrap::compiled::ConfigModel,
            &'static str,
            &'static str,
        ) {
            let config_model = include!(env!("CANIC_CONFIG_MODEL_PATH"));
            let config_source = include_str!(env!("CANIC_CONFIG_SOURCE_PATH"));
            let config_path = env!("CANIC_CONFIG_PATH");
            (config_model, config_source, config_path)
        }

        #[::canic::cdk::init]
        fn init(payload: ::canic::dto::abi::v1::CanisterInitPayload, args: Option<Vec<u8>>) {
            let (config, config_source, config_path) = __canic_compiled_config();

            $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::init_nonroot_canister_before_bootstrap(
                $canister_role,
                payload,
                config,
                config_source,
                config_path,
                cfg!(canic_role_attestation_refresh),
            );

            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:init_block",
                {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_init_nonroot_bootstrap(args.clone());
                    $crate::__canic_start_nonroot_user_timers!(args);
                }
                $(, $init)?
            );
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            let (config, config_source, config_path) = __canic_compiled_config();

            $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::post_upgrade_nonroot_canister_before_bootstrap(
                $canister_role,
                config,
                config_source,
                config_path,
                cfg!(canic_role_attestation_refresh),
            );

            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:post_upgrade_block",
                {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_post_upgrade_nonroot_bootstrap();
                    $crate::__canic_start_nonroot_upgrade_timers!();
                }
                $(, $init)?
            );
        }
    };
}

// Local-dev lifecycle core for standalone sandbox canisters.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_local_lifecycle_core {
    ($canister_role:expr $(, $init:block)?) => {
        #[doc(hidden)]
        fn __canic_compiled_config() -> (
            $crate::__internal::core::bootstrap::compiled::ConfigModel,
            &'static str,
            &'static str,
        ) {
            let config_model = include!(env!("CANIC_CONFIG_MODEL_PATH"));
            let config_source = include_str!(env!("CANIC_CONFIG_SOURCE_PATH"));
            let config_path = env!("CANIC_CONFIG_PATH");
            (config_model, config_source, config_path)
        }

        #[doc(hidden)]
        fn __canic_local_principal(byte: u8) -> ::canic::cdk::types::Principal {
            ::canic::cdk::types::Principal::from_slice(&[byte; 29])
        }

        #[doc(hidden)]
        fn __canic_local_init_payload(
            role: $crate::__internal::core::ids::CanisterRole,
        ) -> ::canic::dto::abi::v1::CanisterInitPayload {
            let root_pid = __canic_local_principal(1);
            let subnet_pid = __canic_local_principal(2);
            ::canic::dto::abi::v1::CanisterInitPayload {
                env: ::canic::dto::env::EnvBootstrapArgs {
                    prime_root_pid: Some(root_pid),
                    subnet_role: Some($crate::__internal::core::ids::SubnetRole::PRIME),
                    subnet_pid: Some(subnet_pid),
                    root_pid: Some(root_pid),
                    canister_role: Some(role),
                    parent_pid: Some(root_pid),
                },
                app_index: ::canic::dto::topology::AppIndexArgs(Vec::new()),
                subnet_index: ::canic::dto::topology::SubnetIndexArgs(Vec::new()),
            }
        }

        #[::canic::cdk::init]
        fn init(args: Option<Vec<u8>>) {
            let (config, config_source, config_path) = __canic_compiled_config();
            let role = $canister_role;
            let payload = __canic_local_init_payload(role.clone());

            $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::init_nonroot_canister_before_bootstrap(
                role,
                payload,
                config,
                config_source,
                config_path,
                cfg!(canic_role_attestation_refresh),
            );

            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:init_block",
                {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_init_nonroot_bootstrap(args.clone());
                    $crate::__canic_start_nonroot_user_timers!(args);
                }
                $(, $init)?
            );
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            let (config, config_source, config_path) = __canic_compiled_config();

            $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::post_upgrade_nonroot_canister_before_bootstrap(
                $canister_role,
                config,
                config_source,
                config_path,
                cfg!(canic_role_attestation_refresh),
            );

            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:post_upgrade_block",
                {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_post_upgrade_nonroot_bootstrap();
                    $crate::__canic_start_nonroot_upgrade_timers!();
                }
                $(, $init)?
            );
        }
    };
}

// Lifecycle core for the root Canic canister.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_root_lifecycle_core {
    ($( $init:block )?) => {
        #[doc(hidden)]
        fn __canic_compiled_config() -> (
            $crate::__internal::core::bootstrap::compiled::ConfigModel,
            &'static str,
            &'static str,
        ) {
            let config_model = include!(env!("CANIC_CONFIG_MODEL_PATH"));
            let config_source = include_str!(env!("CANIC_CONFIG_SOURCE_PATH"));
            let config_path = env!("CANIC_CONFIG_PATH");
            (config_model, config_source, config_path)
        }

        #[doc(hidden)]
        #[cfg(canic_has_root_wasm_store_bootstrap_release_set)]
        fn __canic_embedded_root_wasm_store_bootstrap_release_set(
        ) -> &'static [$crate::__internal::core::bootstrap::EmbeddedRootBootstrapEntry] {
            include!(env!("CANIC_ROOT_WASM_STORE_BOOTSTRAP_RELEASE_SET_PATH"))
        }

        #[doc(hidden)]
        #[cfg(not(canic_has_root_wasm_store_bootstrap_release_set))]
        fn __canic_embedded_root_wasm_store_bootstrap_release_set(
        ) -> &'static [$crate::__internal::core::bootstrap::EmbeddedRootBootstrapEntry] {
            &[]
        }

        #[::canic::cdk::init]
        fn init(identity: ::canic::dto::subnet::SubnetIdentity) {
            let (config, config_source, config_path) = __canic_compiled_config();
            let embedded_wasm_store_bootstrap_release_set =
                __canic_embedded_root_wasm_store_bootstrap_release_set();

            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::init_root_canister_before_bootstrap(
                identity,
                config,
                config_source,
                config_path,
                embedded_wasm_store_bootstrap_release_set,
            );

            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:init_block",
                {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_init_root_bootstrap();
                    $crate::__canic_start_root_user_timers!();
                }
                $(, $init)?
            );
        }

        #[::canic::cdk::post_upgrade]
        fn post_upgrade() {
            let (config, config_source, config_path) = __canic_compiled_config();
            let embedded_wasm_store_bootstrap_release_set =
                __canic_embedded_root_wasm_store_bootstrap_release_set();

            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::post_upgrade_root_canister_before_bootstrap(
                config,
                config_source,
                config_path,
                embedded_wasm_store_bootstrap_release_set,
            );

            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:post_upgrade_block",
                {
                    $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_post_upgrade_root_bootstrap();
                    $crate::__canic_start_root_upgrade_timers!();
                }
                $(, $init)?
            );
        }
    };
}

// Run the optional init block from a lifecycle timer, then schedule continuation timers.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_after_optional_start_init_hook {
    ($label:expr, $after:block) => {{
        $after
    }};
    ($label:expr, $after:block, $init:block) => {{
        $crate::__internal::core::api::timer::TimerApi::set_lifecycle_timer(
            ::core::time::Duration::ZERO,
            $label,
            async move {
                $init
                $after
            },
        );
    }};
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
        $crate::canic_bundle_shared_runtime_endpoints!();
        $crate::canic_bundle_nonroot_only_endpoints!();
    };
}

// Default root capability bundle composition.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_root_capability_bundles {
    () => {
        $crate::canic_bundle_shared_runtime_endpoints!();
        $crate::canic_bundle_root_only_endpoints!();
    };
}

// Ingress inspect-message hook shared by Canic-managed canisters.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_ingress_payload_inspect {
    () => {
        #[::canic::cdk::inspect_message]
        fn canic_inspect_message() {
            $crate::__internal::core::ingress::payload::inspect_update_message();
        }
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
    ($canister_role:expr $(, init = $init:block)? $(,)?) => {
        $crate::__canic_start_nonroot_lifecycle_core!($canister_role $(, $init)?);
        $crate::__canic_start_ingress_payload_inspect!();
        $crate::__canic_start_nonroot_capability_bundles!();
    };
}

/// Configure a local-only non-root Canic canister for manual development.
///
/// `start_local!` is intentionally for standalone dev canisters such as a
/// sandbox. It synthesizes a minimal local environment during `init`, so
/// `icp deploy <canister>` can run without entering the full CANIC bootstrap
/// payload by hand.
///
/// Do not use this macro for production canisters, root-managed child
/// canisters, release-set members, or test fixtures that need real topology
/// metadata. Those should use [`start!`] and receive explicit lifecycle args.
#[macro_export]
macro_rules! start_local {
    ($canister_role:expr $(, init = $init:block)? $(,)?) => {
        $crate::__canic_start_local_lifecycle_core!($canister_role $(, $init)?);
        $crate::__canic_start_ingress_payload_inspect!();
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
    ($(init = $init:block)? $(,)?) => {
        $crate::__canic_start_root_lifecycle_core!($($init)?);
        $crate::__canic_start_ingress_payload_inspect!();
        $crate::__canic_start_root_capability_bundles!();
    };
}

/// Configure lifecycle hooks and the canonical endpoint bundle for a subnet-local
/// `wasm_store` canister.
///
/// This specialized macro exists so downstreams can use the built-in Canic
/// `wasm_store` role without copying the reference canister implementation.
///
/// Unlike the ordinary non-root bundle, this surface intentionally excludes the
/// generic observability and topology-view queries that are not part of the
/// canonical `wasm_store` contract.
#[macro_export]
macro_rules! start_wasm_store {
    ($(init = $init:block)? $(,)?) => {
        #[allow(clippy::unused_async)]
        async fn canic_setup() {}

        #[allow(clippy::unused_async)]
        async fn canic_install(_: Option<Vec<u8>>) {}

        #[allow(clippy::unused_async)]
        async fn canic_upgrade() {}

        $crate::__canic_start_nonroot_lifecycle_core!(
            $crate::api::canister::CanisterRole::WASM_STORE
            $(, $init)?
        );
        $crate::__canic_start_ingress_payload_inspect!();
        $crate::canic_bundle_wasm_store_runtime_endpoints!();
    };
}
