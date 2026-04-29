use crate::{
    config::schema::ConfigModel, dto::abi::v1::CanisterInitPayload, ids::CanisterRole, lifecycle,
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
        with_role_attestation_refresh: bool,
    ) {
        lifecycle::init::nonroot::init_nonroot_canister_before_bootstrap(
            role,
            payload,
            config,
            config_source,
            config_path,
            with_role_attestation_refresh,
        );
    }

    pub fn schedule_init_nonroot_bootstrap(args: Option<Vec<u8>>) {
        lifecycle::init::nonroot::schedule_init_nonroot_bootstrap(args);
    }

    pub fn post_upgrade_nonroot_canister_before_bootstrap(
        role: CanisterRole,
        config: ConfigModel,
        config_source: &str,
        config_path: &str,
        with_role_attestation_refresh: bool,
    ) {
        lifecycle::upgrade::nonroot::post_upgrade_nonroot_canister_before_bootstrap(
            role,
            config,
            config_source,
            config_path,
            with_role_attestation_refresh,
        );
    }

    pub fn schedule_post_upgrade_nonroot_bootstrap() {
        lifecycle::upgrade::nonroot::schedule_post_upgrade_nonroot_bootstrap();
    }
}
