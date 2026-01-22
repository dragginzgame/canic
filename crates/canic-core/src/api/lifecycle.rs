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
    pub fn init_root_canister(identity: SubnetIdentity, config_str: &str, config_path: &str) {
        lifecycle::init::init_root_canister(identity, config_str, config_path);
    }

    pub fn post_upgrade_root_canister(config_str: &str, config_path: &str) {
        lifecycle::upgrade::post_upgrade_root_canister(config_str, config_path);
    }

    pub fn init_nonroot_canister(
        role: CanisterRole,
        payload: CanisterInitPayload,
        args: Option<Vec<u8>>,
        config_str: &str,
        config_path: &str,
    ) {
        lifecycle::init::init_nonroot_canister(role, payload, args, config_str, config_path);
    }

    pub fn post_upgrade_nonroot_canister(role: CanisterRole, config_str: &str, config_path: &str) {
        lifecycle::upgrade::post_upgrade_nonroot_canister(role, config_str, config_path);
    }
}
