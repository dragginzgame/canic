use crate::{
    dto::{abi::v1::CanisterInitPayload, subnet::SubnetIdentity},
    ids::CanisterRole,
    lifecycle,
};

///
/// LifecycleApi
///

pub struct LifecycleApi;

impl LifecycleApi {
    pub fn init_root_canister_before_bootstrap(
        identity: SubnetIdentity,
        config_str: &str,
        config_path: &str,
    ) {
        lifecycle::init::init_root_canister_before_bootstrap(identity, config_str, config_path);
    }

    pub fn schedule_init_root_bootstrap() {
        lifecycle::init::schedule_init_root_bootstrap();
    }

    pub fn post_upgrade_root_canister_before_bootstrap(config_str: &str, config_path: &str) {
        lifecycle::upgrade::post_upgrade_root_canister_before_bootstrap(config_str, config_path);
    }

    pub fn schedule_post_upgrade_root_bootstrap() {
        lifecycle::upgrade::schedule_post_upgrade_root_bootstrap();
    }

    pub fn init_nonroot_canister_before_bootstrap(
        role: CanisterRole,
        payload: CanisterInitPayload,
        config_str: &str,
        config_path: &str,
    ) {
        lifecycle::init::init_nonroot_canister_before_bootstrap(
            role,
            payload,
            config_str,
            config_path,
        );
    }

    pub fn schedule_init_nonroot_bootstrap(args: Option<Vec<u8>>) {
        lifecycle::init::schedule_init_nonroot_bootstrap(args);
    }

    pub fn post_upgrade_nonroot_canister_before_bootstrap(
        role: CanisterRole,
        config_str: &str,
        config_path: &str,
    ) {
        lifecycle::upgrade::post_upgrade_nonroot_canister_before_bootstrap(
            role,
            config_str,
            config_path,
        );
    }

    pub fn schedule_post_upgrade_nonroot_bootstrap() {
        lifecycle::upgrade::schedule_post_upgrade_nonroot_bootstrap();
    }
}
