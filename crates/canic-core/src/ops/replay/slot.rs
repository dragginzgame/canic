use crate::{
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
pub fn upsert_root_slot(key: ReplaySlotKey, record: RootReplayRecord) {
    RootReplayOps::upsert(key, record);
}

/// root_slot_len
///
/// Return the number of replay entries currently stored.
#[must_use]
pub fn root_slot_len() -> usize {
    RootReplayOps::len()
}

/// purge_root_expired
///
/// Purge expired replay records up to the provided scan limit.
pub fn purge_root_expired(now: u64, limit: usize) -> usize {
    RootReplayOps::purge_expired(now, limit)
}
