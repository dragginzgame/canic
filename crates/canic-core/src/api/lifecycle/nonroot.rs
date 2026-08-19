use crate::{
    config::schema::ConfigModel,
    dto::{
        abi::v1::CanisterInitPayload, env::EnvBootstrapArgs,
        fleet_subnet_root::FleetSubnetWasmStoreInitArgs,
    },
    ids::{CanisterRole, ManagedCanisterBinding},
    lifecycle,
};

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    #[doc(hidden)]
    pub fn configure_component_runtime_with_automatic_topup(
        request: crate::dto::component_registry::ComponentRuntimeDirectoryPreparationRequest,
    ) -> Result<
        crate::view::fleet_activation::ComponentRuntimeActivationTransition,
        crate::dto::error::Error,
    > {
        crate::workflow::component_runtime::configure_with_automatic_topup(request)
            .map_err(Into::into)
    }

    /// Return the immutable Registry-issued identity retained by this application Canister.
    pub fn managed_binding() -> Result<ManagedCanisterBinding, crate::dto::error::Error> {
        crate::ops::runtime::env::EnvOps::managed_binding().map_err(Into::into)
    }

    pub fn init_nonroot_canister_before_bootstrap(
        role: CanisterRole,
        payload: CanisterInitPayload,
        application_init_args: Option<Vec<u8>>,
        embedded_release_build_id: Option<&str>,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::nonroot::init_nonroot_canister_before_bootstrap(
            role,
            payload,
            application_init_args,
            embedded_release_build_id,
            config,
            config_source,
            config_path,
        );
    }

    pub fn init_wasm_store_before_bootstrap(
        input: FleetSubnetWasmStoreInitArgs,
        embedded_release_build_id: Option<&str>,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::nonroot::init_wasm_store_before_bootstrap(
            input,
            embedded_release_build_id,
            config,
            config_source,
            config_path,
        );
    }

    pub fn schedule_init_nonroot_bootstrap() {
        lifecycle::init::nonroot::schedule_init_nonroot_bootstrap();
    }

    pub fn init_local_nonroot_canister_before_bootstrap(
        role: CanisterRole,
        env: EnvBootstrapArgs,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::nonroot::init_local_nonroot_canister_before_bootstrap(
            role,
            env,
            config,
            config_source,
            config_path,
        );
    }

    #[doc(hidden)]
    pub fn init_local_nonroot_canister_with_automatic_topup_before_bootstrap(
        role: CanisterRole,
        env: EnvBootstrapArgs,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::nonroot::init_local_nonroot_canister_with_automatic_topup_before_bootstrap(
            role,
            env,
            config,
            config_source,
            config_path,
        );
    }

    #[must_use]
    pub fn post_upgrade_nonroot_canister_before_bootstrap(
        role: CanisterRole,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) -> bool {
        lifecycle::upgrade::nonroot::post_upgrade_nonroot_canister_before_bootstrap(
            role,
            config,
            config_source,
            config_path,
        )
    }

    #[doc(hidden)]
    #[must_use]
    pub fn post_upgrade_nonroot_canister_with_automatic_topup_before_bootstrap(
        role: CanisterRole,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) -> bool {
        lifecycle::upgrade::nonroot::post_upgrade_nonroot_canister_with_automatic_topup_before_bootstrap(
            role,
            config,
            config_source,
            config_path,
        )
    }

    pub fn schedule_post_upgrade_nonroot_bootstrap() {
        lifecycle::upgrade::nonroot::schedule_post_upgrade_nonroot_bootstrap();
    }

    #[must_use]
    pub fn post_upgrade_local_nonroot_canister_before_bootstrap(
        role: CanisterRole,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) -> bool {
        lifecycle::upgrade::nonroot::post_upgrade_local_nonroot_canister_before_bootstrap(
            role,
            config,
            config_source,
            config_path,
        )
    }

    #[doc(hidden)]
    #[must_use]
    pub fn post_upgrade_local_nonroot_canister_with_automatic_topup_before_bootstrap(
        role: CanisterRole,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) -> bool {
        lifecycle::upgrade::nonroot::post_upgrade_local_nonroot_canister_with_automatic_topup_before_bootstrap(
            role,
            config,
            config_source,
            config_path,
        )
    }
}
