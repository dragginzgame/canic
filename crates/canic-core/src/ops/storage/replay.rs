use crate::storage::stable::replay::{ReplaySlotKey, RootReplayRecord, RootReplayStore};

///
/// RootReplayOps
/// Mechanical stable replay store access (no policy).
///

pub struct RootReplayOps;

impl RootReplayOps {
    #[must_use]
    pub fn get(key: ReplaySlotKey) -> Option<RootReplayRecord> {
        RootReplayStore::get(key)
    }

    pub fn upsert(key: ReplaySlotKey, record: RootReplayRecord) {
        RootReplayStore::upsert(key, record);
    }

    #[must_use]
    pub fn len() -> usize {
        RootReplayStore::len()
    }

    pub fn purge_expired(now: u64, limit: usize) -> usize {
        let expired = RootReplayStore::collect_expired(now, limit);
        for key in &expired {
            let _ = RootReplayStore::remove(*key);
        }
        expired.len()
    }
}

#[cfg(test)]
impl RootReplayOps {
    pub fn reset_for_tests() {
        RootReplayStore::reset_for_tests();
    }
}
