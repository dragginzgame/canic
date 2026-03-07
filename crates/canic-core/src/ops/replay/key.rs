use crate::{
    cdk::types::Principal,
    ops::storage::replay::{ReplayService, RootReplayOps},
    storage::stable::replay::ReplaySlotKey,
};
use sha2::{Digest, Sha256};

const ROOT_REPLAY_NONCE: [u8; 16] = [0u8; 16];
const LEGACY_ROOT_REPLAY_SLOT_KEY_DOMAIN: &[u8] = b"root-replay-slot-key:v1";

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

/// legacy_root_slot_key
///
/// Build the legacy 0.11-era replay key for compatibility reads.
#[must_use]
pub fn legacy_root_slot_key(
    caller: Principal,
    subnet_id: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    let mut hasher = Sha256::new();
    hasher.update((LEGACY_ROOT_REPLAY_SLOT_KEY_DOMAIN.len() as u64).to_be_bytes());
    hasher.update(LEGACY_ROOT_REPLAY_SLOT_KEY_DOMAIN);
    hasher.update(caller.as_slice());
    hasher.update(subnet_id.as_slice());
    hasher.update(request_id);
    ReplaySlotKey(hasher.finalize().into())
}
