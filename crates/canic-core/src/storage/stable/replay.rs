use crate::{
    cdk::structures::{
        BTreeMap, DefaultMemoryImpl, Storable, memory::VirtualMemory, storable::Bound,
    },
    eager_static, ic_memory,
    memory::impl_storable_unbounded,
    storage::{prelude::*, stable::memory::auth::ROOT_REPLAY_ID},
};
use std::{borrow::Cow, cell::RefCell};

eager_static! {
    static ROOT_REPLAY: RefCell<
        BTreeMap<ReplaySlotKey, RootReplayRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(RootReplayStore, ROOT_REPLAY_ID)),
    );
}

///
/// ReplaySlotKey
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ReplaySlotKey(pub [u8; 32]);

impl Storable for ReplaySlotKey {
    const BOUND: Bound = Bound::Bounded {
        max_size: 32,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.0.to_vec())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        let mut out = [0u8; 32];

        if bytes.len() == 32 {
            out.copy_from_slice(bytes);
        }

        Self(out)
    }
}

///
/// RootReplayRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootReplayRecord {
    pub payload_hash: [u8; 32],
    pub issued_at: u64,
    pub expires_at: u64,
    pub response_candid: Vec<u8>,
}

impl_storable_unbounded!(RootReplayRecord);

///
/// RootReplayStore
///

pub struct RootReplayStore;

impl RootReplayStore {
    #[must_use]
    pub(crate) fn get(key: ReplaySlotKey) -> Option<RootReplayRecord> {
        ROOT_REPLAY.with_borrow(|map| map.get(&key))
    }

    pub(crate) fn upsert(key: ReplaySlotKey, record: RootReplayRecord) {
        ROOT_REPLAY.with_borrow_mut(|map| {
            map.insert(key, record);
        });
    }

    pub(crate) fn remove(key: ReplaySlotKey) -> Option<RootReplayRecord> {
        ROOT_REPLAY.with_borrow_mut(|map| map.remove(&key))
    }

    #[must_use]
    pub(crate) fn len() -> usize {
        ROOT_REPLAY.with_borrow(|map| usize::try_from(map.len()).unwrap_or(usize::MAX))
    }

    pub(crate) fn collect_expired(now: u64, limit: usize) -> Vec<ReplaySlotKey> {
        let mut expired = Vec::new();
        ROOT_REPLAY.with_borrow(|map| {
            for entry in map.iter() {
                if entry.value().expires_at < now {
                    expired.push(*entry.key());
                    if expired.len() >= limit {
                        break;
                    }
                }
            }
        });
        expired
    }
}

#[cfg(test)]
impl RootReplayStore {
    pub(crate) fn reset_for_tests() {
        ROOT_REPLAY.with_borrow_mut(BTreeMap::clear);
    }
}
