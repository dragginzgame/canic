use crate::{
    dto::{abi::v1::CanisterInitPayload, subnet::SubnetIdentity},
    ids::CanisterRole,
    lifecycle,
};

///
/// Lifecycle API
///

pub fn init_root_canister(identity: SubnetIdentity) {
    lifecycle::init::init_root_canister(identity);
}

pub fn post_upgrade_root_canister() {
    lifecycle::upgrade::post_upgrade_root_canister();
}

pub fn init_nonroot_canister(
    role: CanisterRole,
    payload: CanisterInitPayload,
    args: Option<Vec<u8>>,
) {
    lifecycle::init::init_nonroot_canister(role, payload, args);
}

pub fn post_upgrade_nonroot_canister(role: CanisterRole) {
    lifecycle::upgrade::post_upgrade_nonroot_canister(role);
}
