use canic_core::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap::{EmbeddedRootBootstrapEntry, compiled::ConfigModel},
    dto::subnet::SubnetIdentity,
};
use std::time::Duration;

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    /// Delegate root init-time runtime seeding to the current core implementation.
    pub fn init_root_canister_before_bootstrap(
        identity: SubnetIdentity,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
        embedded_wasm_store_bootstrap_release_set: &'static [EmbeddedRootBootstrapEntry],
    ) {
        crate::api::template::WasmStoreBootstrapApi::register_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
        crate::runtime::install::register_template_module_source_resolver();
        canic_core::api::lifecycle::root::LifecycleApi::init_root_canister_before_bootstrap(
            identity,
            config,
            config_source,
            config_path,
        );
        crate::api::template::WasmStoreBootstrapApi::log_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
    }

    /// Delegate root init-time bootstrap scheduling to the current core implementation.
    pub fn schedule_init_root_bootstrap() {
        LifecycleMetricsApi::record_bootstrap(
            LifecycleMetricPhase::Init,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Scheduled,
        );

        canic_core::api::timer::TimerApi::set_lifecycle_timer(
            Duration::ZERO,
            "canic:bootstrap:init_root_canister",
            async {
                crate::workflow::bootstrap::root::bootstrap_init_root_canister().await;
            },
        );
    }

    /// Delegate root post-upgrade runtime restore to the current core implementation.
    pub fn post_upgrade_root_canister_before_bootstrap(
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
        embedded_wasm_store_bootstrap_release_set: &'static [EmbeddedRootBootstrapEntry],
    ) {
        crate::api::template::WasmStoreBootstrapApi::register_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
        crate::runtime::install::register_template_module_source_resolver();
        canic_core::api::lifecycle::root::LifecycleApi::post_upgrade_root_canister_before_bootstrap(
            config,
            config_source,
            config_path,
        );
        crate::api::template::WasmStoreBootstrapApi::log_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
    }

    /// Delegate root post-upgrade bootstrap scheduling to the current core implementation.
    pub fn schedule_post_upgrade_root_bootstrap() {
        LifecycleMetricsApi::record_bootstrap(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Scheduled,
        );

        canic_core::api::timer::TimerApi::set_lifecycle_timer(
            Duration::ZERO,
            "canic:bootstrap:post_upgrade_root_canister",
            async {
                crate::workflow::bootstrap::root::bootstrap_post_upgrade_root_canister().await;
            },
        );
    }
}
