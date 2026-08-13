// -----------------------------------------------------------------------------
// Start macros
// -----------------------------------------------------------------------------

// Lifecycle core for non-root Canic canisters.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_nonroot_lifecycle_core {
    ($canister_role:expr $(, $init:block)?) => {
        ::std::thread_local! {
            static __CANIC_PREPARED_APPLICATION_INIT_SCHEDULED:
                ::std::cell::Cell<bool> = const { ::std::cell::Cell::new(false) };
        }

        // The activation adapter owns execution of this application hook.
        // Keep the contract bound without polling or scheduling it in Prepared.
        #[doc(hidden)]
        const _: () = {
            let _ = canic_install;
        };

        #[doc(hidden)]
        fn __canic_schedule_prepared_activation_init(args: Option<Vec<u8>>) {
            if __CANIC_PREPARED_APPLICATION_INIT_SCHEDULED.replace(true) {
                return;
            }
            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:prepared_activation_block",
                {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_init_nonroot_bootstrap();
                    $crate::__internal::core::api::timer::TimerApi::defer_lifecycle(
                        ::core::time::Duration::ZERO,
                        "canic:user:init",
                        async move {
                            canic_setup().await;
                            canic_install(args).await;
                        },
                    );
                }
                $(, $init)?
            );
        }

        #[doc(hidden)]
        fn __canic_compiled_config() -> (
            $crate::__internal::core::bootstrap::compiled::ConfigModel,
            &'static str,
            &'static str,
        ) {
            let config_model = include!(env!("CANIC_CONFIG_MODEL_PATH"));
            let config_source = include_str!(env!("CANIC_CONFIG_SOURCE_PATH"));
            let config_path = env!("CANIC_CONFIG_ORIGIN_PATH");
            (config_model, config_source, config_path)
        }

        #[$crate::__internal::cdk::init]
        fn init(payload: ::canic::dto::abi::v1::CanisterInitPayload, args: Option<Vec<u8>>) {
            let (config, config_source, config_path) = __canic_compiled_config();

            $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::init_nonroot_canister_before_bootstrap(
                $canister_role,
                payload,
                args,
                option_env!("CANIC_RELEASE_BUILD_ID"),
                config,
                config_source,
                config_path,
            );
        }

        #[$crate::__internal::cdk::post_upgrade]
        fn post_upgrade() {
            let (config, config_source, config_path) = __canic_compiled_config();

            let active = $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::post_upgrade_nonroot_canister_before_bootstrap(
                $canister_role,
                config,
                config_source,
                config_path,
            );

            if active {
                $crate::__canic_after_optional_start_init_hook!(
                    "canic:user:post_upgrade_block",
                    {
                        $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_post_upgrade_nonroot_bootstrap();
                        $crate::__internal::core::api::timer::TimerApi::defer_lifecycle(
                            ::core::time::Duration::ZERO,
                            "canic:user:post_upgrade",
                            async move {
                                canic_setup().await;
                                canic_upgrade().await;
                            },
                        );
                    }
                    $(, $init)?
                );
            }
        }
    };
}

// Lifecycle core for the host-installed sibling Wasm Store.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_wasm_store_lifecycle_core {
    ($(, $init:block)?) => {
        ::std::thread_local! {
            static __CANIC_PREPARED_APPLICATION_INIT_SCHEDULED:
                ::std::cell::Cell<bool> = const { ::std::cell::Cell::new(false) };
        }

        #[doc(hidden)]
        const _: () = {
            let _ = canic_install;
        };

        #[doc(hidden)]
        fn __canic_schedule_prepared_activation_init(args: Option<Vec<u8>>) {
            if __CANIC_PREPARED_APPLICATION_INIT_SCHEDULED.replace(true) {
                return;
            }
            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:prepared_activation_block",
                {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_init_nonroot_bootstrap();
                    $crate::__internal::core::api::timer::TimerApi::defer_lifecycle(
                        ::core::time::Duration::ZERO,
                        "canic:user:init",
                        async move {
                            canic_setup().await;
                            canic_install(args).await;
                        },
                    );
                }
                $(, $init)?
            );
        }

        #[doc(hidden)]
        fn __canic_compiled_config() -> (
            $crate::__internal::core::bootstrap::compiled::ConfigModel,
            &'static str,
            &'static str,
        ) {
            let config_model = include!(env!("CANIC_CONFIG_MODEL_PATH"));
            let config_source = include_str!(env!("CANIC_CONFIG_SOURCE_PATH"));
            let config_path = env!("CANIC_CONFIG_ORIGIN_PATH");
            (config_model, config_source, config_path)
        }

        #[$crate::__internal::cdk::init]
        fn init(args: ::canic::dto::fleet_subnet_root::FleetSubnetWasmStoreInitArgs) {
            let (config, config_source, config_path) = __canic_compiled_config();
            $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::init_wasm_store_before_bootstrap(
                args,
                option_env!("CANIC_RELEASE_BUILD_ID"),
                config,
                config_source,
                config_path,
            );
        }

        #[$crate::__internal::cdk::post_upgrade]
        fn post_upgrade() {
            let (config, config_source, config_path) = __canic_compiled_config();
            let active = $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::post_upgrade_nonroot_canister_before_bootstrap(
                $crate::api::canister::CanisterRole::WASM_STORE,
                config,
                config_source,
                config_path,
            );

            if active {
                $crate::__canic_after_optional_start_init_hook!(
                    "canic:user:post_upgrade_block",
                    {
                        $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_post_upgrade_nonroot_bootstrap();
                        $crate::__internal::core::api::timer::TimerApi::defer_lifecycle(
                            ::core::time::Duration::ZERO,
                            "canic:user:post_upgrade",
                            async move {
                                canic_setup().await;
                                canic_upgrade().await;
                            },
                        );
                    }
                    $(, $init)?
                );
            }
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
            let config_path = env!("CANIC_CONFIG_ORIGIN_PATH");
            (config_model, config_source, config_path)
        }

        #[doc(hidden)]
        fn __canic_local_principal(byte: u8) -> $crate::__internal::cdk::Principal {
            $crate::__internal::cdk::Principal::from_slice(&[byte; 29])
        }

        #[doc(hidden)]
        fn __canic_local_env(
            role: $crate::__internal::core::ids::CanisterRole,
            component_spec: $crate::__internal::core::ids::ComponentSpecId,
        ) -> ::canic::dto::env::EnvBootstrapArgs {
            let root_pid = __canic_local_principal(1);
            let subnet_pid = __canic_local_principal(2);
            ::canic::dto::env::EnvBootstrapArgs {
                fleet_subnet_root_pid: Some(root_pid),
                component_spec: Some(component_spec),
                subnet_pid: Some(subnet_pid),
                root_pid: Some(root_pid),
                canister_role: Some(role),
                parent_pid: Some(root_pid),
            }
        }

        #[$crate::__internal::cdk::init]
        fn init(args: Option<Vec<u8>>) {
            let (config, config_source, config_path) = __canic_compiled_config();
            let role = $canister_role;
            let component_spec = config
                .component_spec_for_role(&role)
                .map(|(component_spec, _config)| component_spec.clone())
                .expect("local bootstrap role must belong to exactly one Component Spec");
            let env = __canic_local_env(role.clone(), component_spec);

            $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::init_local_nonroot_canister_before_bootstrap(
                role,
                env,
                config,
                config_source,
                config_path,
            );

            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:init_block",
                {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_init_nonroot_bootstrap();
                    $crate::__internal::core::api::timer::TimerApi::defer_lifecycle(
                        ::std::time::Duration::ZERO,
                        "canic:user:init",
                        async move {
                            canic_setup().await;
                            canic_install(args).await;
                        },
                    );
                }
                $(, $init)?
            );
        }

        #[$crate::__internal::cdk::post_upgrade]
        fn post_upgrade() {
            let (config, config_source, config_path) = __canic_compiled_config();

            let _active = $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::post_upgrade_local_nonroot_canister_before_bootstrap(
                $canister_role,
                config,
                config_source,
                config_path,
            );

            $crate::__canic_after_optional_start_init_hook!(
                "canic:user:post_upgrade_block",
                {
                    $crate::__internal::core::api::lifecycle::nonroot::LifecycleApi::schedule_post_upgrade_nonroot_bootstrap();
                    $crate::__internal::core::api::timer::TimerApi::defer_lifecycle(
                        ::core::time::Duration::ZERO,
                        "canic:user:post_upgrade",
                        async move {
                            canic_setup().await;
                            canic_upgrade().await;
                        },
                    );
                }
                $(, $init)?
            );
        }
    };
}

// Lifecycle core for the root Canic canister.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_root_lifecycle_core {
    ($( $init:block )?) => {
        ::std::thread_local! {
            static __CANIC_PREPARED_ROOT_INIT_COMPLETED:
                ::std::cell::Cell<bool> = const { ::std::cell::Cell::new(false) };
            static __CANIC_PREPARED_APPLICATION_INIT_SCHEDULED:
                ::std::cell::Cell<bool> = const { ::std::cell::Cell::new(false) };
        }

        // The activation adapter owns execution of this application hook.
        // Keep the contract bound without polling or scheduling it in Prepared.
        #[doc(hidden)]
        const _: () = {
            let _ = canic_install;
        };

        #[doc(hidden)]
        async fn __canic_run_prepared_root_init_block() {
            if __CANIC_PREPARED_ROOT_INIT_COMPLETED.replace(true) {
                return;
            }
            $($init)?
        }

        #[doc(hidden)]
        fn __canic_schedule_prepared_activation_init() {
            if __CANIC_PREPARED_APPLICATION_INIT_SCHEDULED.replace(true) {
                return;
            }
            $crate::__internal::core::api::timer::TimerApi::defer_lifecycle(
                ::core::time::Duration::ZERO,
                "canic:user:init",
                async move {
                    canic_setup().await;
                    canic_install().await;
                },
            );
        }

        #[doc(hidden)]
        fn __canic_compiled_config() -> (
            $crate::__internal::core::bootstrap::compiled::ConfigModel,
            &'static str,
            &'static str,
        ) {
            let config_model = include!(env!("CANIC_CONFIG_MODEL_PATH"));
            let config_source = include_str!(env!("CANIC_CONFIG_SOURCE_PATH"));
            let config_path = env!("CANIC_CONFIG_ORIGIN_PATH");
            (config_model, config_source, config_path)
        }

        #[$crate::__internal::cdk::init]
        fn init(args: ::canic::dto::fleet_subnet_root::FleetSubnetRootInitArgs) {
            let (config, config_source, config_path) = __canic_compiled_config();

            $crate::__internal::control_plane::api::lifecycle::LifecycleApi::init_root_canister_before_bootstrap(
                args,
                option_env!("CANIC_RELEASE_BUILD_ID"),
                config,
                config_source,
                config_path,
            );
        }

        #[$crate::__internal::cdk::post_upgrade]
        fn post_upgrade() {
            let (config, config_source, config_path) = __canic_compiled_config();

            let active = $crate::__internal::control_plane::api::lifecycle::LifecycleApi::post_upgrade_root_canister_before_bootstrap(
                config,
                config_source,
                config_path,
            );

            if active {
                $crate::__canic_after_optional_start_init_hook!(
                    "canic:user:post_upgrade_block",
                    {
                        $crate::__internal::control_plane::api::lifecycle::LifecycleApi::schedule_post_upgrade_root_bootstrap();
                        $crate::__internal::core::api::timer::TimerApi::defer_lifecycle(
                            ::core::time::Duration::ZERO,
                            "canic:user:post_upgrade",
                            async move {
                                canic_setup().await;
                                canic_upgrade().await;
                            },
                        );
                    }
                    $(, $init)?
                );
            }
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
        $crate::__internal::core::api::timer::TimerApi::defer_lifecycle(
            ::core::time::Duration::ZERO,
            $label,
            async move {
                $init
                $after
            },
        );
    }};
}

// Ingress inspect-message hook shared by Canic-managed canisters.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_start_ingress_payload_inspect {
    () => {
        #[$crate::__internal::cdk::inspect_message]
        fn canic_inspect_message() {
            $crate::__internal::core::ingress::payload::inspect_update_message();
        }
    };
}

// Require canisters using the Canic lifecycle macros to close the file with
// `canic::finish!()`. Rust item order does not affect this marker lookup, while
// Candid export order still requires the finish macro to appear after endpoints.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_require_finish {
    () => {
        #[doc(hidden)]
        const _: () = __canic_missing_finish_macro_add_canic_finish_at_end_after_all_endpoints;
    };
}

/// Finish a Canic canister module.
///
/// Place this macro at the end of the canister's crate root after
/// `start!`, `start_local!`, `start_wasm_store!`, or
/// `start_fleet_coordinator!` and after any extra endpoint definitions. In
/// local-network builds it exports Candid from the exact selected Wasm; IC
/// builds only satisfy the required Canic finish marker.
#[macro_export]
macro_rules! finish {
    () => {
        #[doc(hidden)]
        const __canic_missing_finish_macro_add_canic_finish_at_end_after_all_endpoints: () = ();

        #[doc(hidden)]
        mod __canic_candid_export {
            #![allow(
                unexpected_cfgs,
                reason = "Canic host builds inject and register this destination-crate cfg"
            )]

            #[cfg(canic_export_candid)]
            $crate::__internal::cdk::export_candid!();
        }
    };
}

/// Configure lifecycle hooks for Canic canisters.
///
/// The canister role comes from `[package.metadata.canic] role = "..."` in the
/// crate manifest and is emitted by `canic::build!` at compile time.
/// `role = "root"` selects root lifecycle adapters and endpoint bundles;
/// every other role selects non-root lifecycle adapters and endpoint bundles.
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
    ($(init = $init:block)? $(,)?) => {
        $crate::__canic_require_finish!();

        #[doc(hidden)]
        #[used]
        static __CANIC_RELEASE_BUILD_ID: &str =
            match option_env!("CANIC_RELEASE_BUILD_ID") {
                Some(value) => value,
                None => "",
            };

        #[cfg(canic_is_root)]
        $crate::__canic_root_lifecycle_core!($($init)?);

        #[cfg(not(canic_is_root))]
        $crate::__canic_start_nonroot_lifecycle_core!(
            $crate::__internal::core::ids::CanisterRole::from(env!("CANIC_CANISTER_ROLE"))
            $(, $init)?
        );

        $crate::__canic_start_ingress_payload_inspect!();

        $crate::canic_bundle_shared_runtime_endpoints!();

        #[cfg(not(canic_is_root))]
        $crate::canic_bundle_managed_nonroot_only_endpoints!();

        #[cfg(canic_is_root)]
        $crate::canic_bundle_root_only_endpoints!();
    };
}

/// Configure a local-only non-root Canic canister for manual development.
///
/// The canister role comes from `[package.metadata.canic] role = "..."` in the
/// crate manifest and is emitted by `canic::build!` at compile time.
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
    ($(init = $init:block)? $(,)?) => {
        $crate::__canic_require_finish!();
        #[cfg(canic_is_root)]
        compile_error!("canic::start_local!() cannot be used for root canisters; use canic::start!()");
        $crate::__canic_start_local_lifecycle_core!(
            $crate::__internal::core::ids::CanisterRole::from(env!("CANIC_CANISTER_ROLE"))
            $(, $init)?
        );
        $crate::__canic_start_ingress_payload_inspect!();
        $crate::canic_bundle_shared_runtime_endpoints!();
        $crate::canic_bundle_local_nonroot_only_endpoints!();
    };
}

/// Configure lifecycle hooks and the canonical endpoint bundle for a Fleet
/// Subnet Root-local `wasm_store` canister.
///
/// This specialized macro exists so downstreams can use the built-in Canic
/// `wasm_store` role without copying the reference canister implementation.
///
/// Unlike the ordinary non-root bundle, this surface intentionally excludes most
/// generic observability and topology-view queries that are not part of the
/// canonical `wasm_store` contract. It still exposes the standard cycle tracker
/// so fleet metrics can treat the store like every other managed canister.
#[macro_export]
macro_rules! start_wasm_store {
    ($(init = $init:block)? $(,)?) => {
        $crate::__canic_require_finish!();
        #[doc(hidden)]
        #[used]
        static __CANIC_RELEASE_BUILD_ID: &str =
            match option_env!("CANIC_RELEASE_BUILD_ID") {
                Some(value) => value,
                None => "",
            };
        #[expect(clippy::unused_async)]
        async fn canic_setup() {}

        #[expect(clippy::unused_async)]
        async fn canic_install(_: Option<Vec<u8>>) {}

        #[expect(clippy::unused_async)]
        async fn canic_upgrade() {}

        $crate::__canic_start_wasm_store_lifecycle_core!($(, $init)?);
        $crate::__canic_start_ingress_payload_inspect!();
        $crate::canic_bundle_wasm_store_runtime_endpoints!();
    };
}

/// Configure the dedicated built-in Fleet Coordinator canister surface.
///
/// The Coordinator is infrastructure outside App roles and Component
/// topology. Its init payload installs one protected Fleet authority and
/// canonical Component Topology, and its endpoint bundle exposes only
/// Coordinator-owned Fleet Registry state.
#[macro_export]
macro_rules! start_fleet_coordinator {
    () => {
        $crate::__canic_require_finish!();
        #[doc(hidden)]
        #[used]
        static __CANIC_RELEASE_BUILD_ID: &str = match option_env!("CANIC_RELEASE_BUILD_ID") {
            Some(value) => value,
            None => "",
        };

        #[$crate::__internal::cdk::init]
        fn init(args: ::canic::dto::fleet_coordinator::FleetCoordinatorInitArgs) {
            $crate::__internal::control_plane::api::fleet_coordinator::FleetCoordinatorApi::init(
                args,
            );
        }

        $crate::__canic_start_ingress_payload_inspect!();
        $crate::canic_emit_fleet_coordinator_endpoints!();
    };
}
