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
/// Persist a fresh replay reservation using the canonical root replay schema.
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

/// purge_root_expired
///
/// Purge expired replay records up to the provided scan limit.
pub fn purge_root_expired(now: u64, limit: usize) -> usize {
    RootReplayOps::purge_expired(now, limit)
}
