use crate::{
    ops::replay::guard::ReplayPending,
    ops::storage::replay::RootReplayOps,
    storage::stable::replay::{ReplaySlotKey, RootReplayRecord},
};

/// get_root_slot
///
/// Read a replay record from the root replay store.
#[must_use]
pub fn get_root_slot(key: ReplaySlotKey) -> Option<RootReplayRecord> {
    RootReplayOps::get(key)
}

/// upsert_root_slot
///
/// Insert or replace a replay record in the root replay store.
#[cfg(test)]
pub fn upsert_root_slot(key: ReplaySlotKey, record: RootReplayRecord) {
    RootReplayOps::upsert(key, record);
}

/// commit_root_slot
///
/// Persist a reserved replay reservation marker before capability execution.
pub fn reserve_root_slot(pending: ReplayPending) {
    RootReplayOps::upsert(
        pending.slot_key,
        RootReplayRecord {
            payload_hash: pending.payload_hash,
            issued_at: pending.issued_at,
            expires_at: pending.expires_at,
            response_candid: vec![],
        },
    );
}

/// commit_root_slot
///
/// Persist canonical replay response bytes for an already-reserved replay slot.
pub fn commit_root_slot(pending: ReplayPending, response_candid: Vec<u8>) {
    RootReplayOps::upsert(
        pending.slot_key,
        RootReplayRecord {
            payload_hash: pending.payload_hash,
            issued_at: pending.issued_at,
            expires_at: pending.expires_at,
            response_candid,
        },
    );
}

/// root_slot_len
///
/// Return the number of replay entries currently stored.
#[must_use]
pub fn root_slot_len() -> usize {
    RootReplayOps::len()
}

/// has_root_slot
///
/// Return whether a replay slot already exists in stable replay storage.
#[must_use]
pub fn has_root_slot(key: ReplaySlotKey) -> bool {
    get_root_slot(key).is_some()
}

/// purge_root_expired
///
/// Purge expired replay records up to the provided scan limit.
pub fn purge_root_expired(now: u64, limit: usize) -> usize {
    RootReplayOps::purge_expired(now, limit)
}

/// remove_root_slot
///
/// Remove a replay slot entry from stable replay storage.
pub fn remove_root_slot(key: ReplaySlotKey) -> Option<RootReplayRecord> {
    RootReplayOps::remove(key)
}
