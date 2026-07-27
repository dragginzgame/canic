use canic_core::{
    api::lifecycle::metrics::{
        LifecycleMetricOutcome, LifecycleMetricPhase, LifecycleMetricRole, LifecycleMetricsApi,
    },
    bootstrap::{EmbeddedRootBootstrapEntry, compiled::ConfigModel},
    control_plane_support::view::fleet_activation::FleetActivationTransition,
    dto::fleet_activation::{FleetActivationResumeRequest, FleetActivationStatusResponse},
    dto::fleet_registry::{
        FleetSubnetRootRegistrySyncRequest, FleetSubnetRootRegistrySyncResponse,
    },
    dto::fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetRootInitArgs},
};
use std::time::Duration;

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    /// Delegate root init-time runtime seeding to the current core implementation.
    pub fn init_root_canister_before_bootstrap(
        args: FleetSubnetRootInitArgs,
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
            args,
            config,
            config_source,
            config_path,
        );
        crate::api::template::WasmStoreBootstrapApi::log_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
    }

    pub fn fleet_subnet_root_authority()
    -> Result<FleetSubnetRootAuthority, canic_core::dto::error::Error> {
        canic_core::api::fleet_activation::FleetActivationApi::root_authority()
    }

    pub async fn synchronize_fleet_registry(
        request: FleetSubnetRootRegistrySyncRequest,
    ) -> Result<FleetSubnetRootRegistrySyncResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_registry_mirror::synchronize(request)
            .await
            .map_err(Into::into)
    }

    pub async fn fleet_registry_sync_status(
        request: FleetSubnetRootRegistrySyncRequest,
    ) -> Result<FleetSubnetRootRegistrySyncResponse, canic_core::dto::error::Error> {
        crate::workflow::fleet_registry_mirror::status(request)
            .await
            .map_err(Into::into)
    }

    pub async fn prepare_fleet_activation()
    -> Result<FleetActivationStatusResponse, canic_core::dto::error::Error> {
        crate::workflow::bootstrap::root::bootstrap_init_root_canister().await;
        if !crate::workflow::bootstrap::root::activation_preparation_complete() {
            return Err(canic_core::dto::error::Error::unavailable(
                "root bootstrap has not prepared the complete managed inventory; inspect bootstrap status and retry activation preparation",
            ));
        }
        canic_core::api::fleet_activation::FleetActivationApi::prepare_root().await
    }

    pub async fn resume_fleet_activation(
        request: FleetActivationResumeRequest,
    ) -> Result<FleetActivationTransition, canic_core::dto::error::Error> {
        let transition =
            canic_core::api::fleet_activation::FleetActivationApi::resume_root(request).await?;
        crate::workflow::bootstrap::root::bootstrap_init_root_canister().await;
        if !canic_core::api::ready::ReadyApi::is_ready() {
            return Err(canic_core::dto::error::Error::unavailable(
                "Fleet activation completed but root bootstrap is not ready; inspect bootstrap status and retry activation resume",
            ));
        }
        Ok(transition)
    }

    /// Delegate root post-upgrade runtime restore to the current core implementation.
    #[must_use]
    pub fn post_upgrade_root_canister_before_bootstrap(
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
        embedded_wasm_store_bootstrap_release_set: &'static [EmbeddedRootBootstrapEntry],
    ) -> bool {
        crate::api::template::WasmStoreBootstrapApi::register_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
        crate::runtime::install::register_template_module_source_resolver();
        let active =
            canic_core::api::lifecycle::root::LifecycleApi::post_upgrade_root_canister_before_bootstrap(
                config,
                config_source,
                config_path,
            );
        crate::api::template::WasmStoreBootstrapApi::log_embedded_root_wasm_store_release_set(
            embedded_wasm_store_bootstrap_release_set,
        );
        active
    }

    /// Delegate root post-upgrade bootstrap scheduling to the current core implementation.
    pub fn schedule_post_upgrade_root_bootstrap() {
        LifecycleMetricsApi::record_bootstrap(
            LifecycleMetricPhase::PostUpgrade,
            LifecycleMetricRole::Root,
            LifecycleMetricOutcome::Scheduled,
        );

        canic_core::api::timer::TimerApi::defer_lifecycle(
            Duration::ZERO,
            "canic:bootstrap:post_upgrade_root_canister",
            async {
                crate::workflow::bootstrap::root::bootstrap_post_upgrade_root_canister().await;
            },
        );
    }
}
