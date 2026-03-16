use crate::{
    cdk::types::Principal,
    ops::storage::replay::{ReplayService, RootReplayOps},
    storage::stable::replay::ReplaySlotKey,
};

const ROOT_REPLAY_NONCE: [u8; 16] = [0u8; 16];

/// root_slot_key
///
/// Build the canonical replay slot key for root request replay tracking.
#[must_use]
pub fn root_slot_key(
    caller: Principal,
    target_canister: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    RootReplayOps::slot_key(
        caller,
        target_canister,
        ReplayService::Root,
        &request_id,
        ROOT_REPLAY_NONCE,
    )
}
