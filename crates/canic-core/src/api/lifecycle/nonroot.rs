use crate::{
    config::schema::ConfigModel,
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        topology::{FleetDirectoryInput, SubnetDirectoryInput},
    },
    ids::CanisterRole,
    lifecycle,
};

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    pub fn init_nonroot_canister_before_bootstrap(
        role: CanisterRole,
        payload: CanisterInitPayload,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::nonroot::init_nonroot_canister_before_bootstrap(
            role,
            payload,
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
        fleet_directory: FleetDirectoryInput,
        subnet_directory: SubnetDirectoryInput,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
    ) {
        lifecycle::init::nonroot::init_local_nonroot_canister_before_bootstrap(
            role,
            env,
            fleet_directory,
            subnet_directory,
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
