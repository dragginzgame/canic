use crate::{
    config::schema::ConfigModel,
    dto::{abi::v1::CanisterInitPayload, env::EnvBootstrapArgs},
    ids::{CanisterRole, ManagedCanisterBinding},
    lifecycle,
};

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    /// Return the immutable Registry-issued identity retained by this application Canister.
    pub fn managed_binding() -> Result<ManagedCanisterBinding, crate::dto::error::Error> {
        crate::ops::runtime::env::EnvOps::managed_binding().map_err(Into::into)
    }

    pub fn init_nonroot_canister_before_bootstrap(
        role: CanisterRole,
        payload: CanisterInitPayload,
        application_init_args: Option<Vec<u8>>,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::nonroot::init_nonroot_canister_before_bootstrap(
            role,
            payload,
            application_init_args,
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
}
