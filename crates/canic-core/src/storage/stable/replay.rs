use crate::{
    cdk::structures::{
        BTreeMap, DefaultMemoryImpl, Storable, memory::VirtualMemory, storable::Bound,
    },
    cdk::types::Principal,
    eager_static, ic_memory,
    storage::{prelude::*, stable::memory::auth::ROOT_REPLAY_ID},
};
use std::{borrow::Cow, cell::RefCell};

const ROOT_REPLAY_RECORD_MIN_BYTES: usize = 1 + 32 + 8 + 8 + 4;

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
    pub caller: Principal,
    pub payload_hash: [u8; 32],
    pub issued_at: u64,
    pub expires_at: u64,
    pub response_bytes: Vec<u8>,
}

impl Storable for RootReplayRecord {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.clone().into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let caller = self.caller.as_slice();
        let caller_len = u8::try_from(caller.len()).expect("root replay caller principal fits u8");
        let response_len =
            u32::try_from(self.response_bytes.len()).expect("root replay response bytes fit u32");
        let mut bytes = Vec::with_capacity(
            ROOT_REPLAY_RECORD_MIN_BYTES
                + caller.len()
                + usize::try_from(response_len).unwrap_or(usize::MAX),
        );
        bytes.push(caller_len);
        bytes.extend_from_slice(caller);
        bytes.extend_from_slice(&self.payload_hash);
        bytes.extend_from_slice(&self.issued_at.to_le_bytes());
        bytes.extend_from_slice(&self.expires_at.to_le_bytes());
        bytes.extend_from_slice(&response_len.to_le_bytes());
        bytes.extend_from_slice(&self.response_bytes);
        bytes
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let bytes = bytes.as_ref();
        assert!(
            bytes.len() >= ROOT_REPLAY_RECORD_MIN_BYTES,
            "root replay record shorter than minimum header"
        );

        let caller_len = bytes[0] as usize;
        let caller_start = 1usize;
        let caller_end = caller_start
            .checked_add(caller_len)
            .expect("root replay caller length overflow");
        assert!(
            bytes.len() >= caller_end + 32 + 8 + 8 + 4,
            "root replay record shorter than caller and fixed fields"
        );
        let caller = Principal::from_slice(&bytes[caller_start..caller_end]);

        let mut payload_hash = [0u8; 32];
        let payload_hash_end = caller_end + 32;
        payload_hash.copy_from_slice(&bytes[caller_end..payload_hash_end]);

        let issued_at = u64::from_le_bytes(
            bytes[payload_hash_end..payload_hash_end + 8]
                .try_into()
                .expect("root replay record issued_at"),
        );
        let expires_at_start = payload_hash_end + 8;
        let expires_at = u64::from_le_bytes(
            bytes[expires_at_start..expires_at_start + 8]
                .try_into()
                .expect("root replay record expires_at"),
        );
        let response_len_start = expires_at_start + 8;
        let response_len = u32::from_le_bytes(
            bytes[response_len_start..response_len_start + 4]
                .try_into()
                .expect("root replay record response length"),
        ) as usize;
        let response_start = response_len_start + 4;
        let response_end = response_start
            .checked_add(response_len)
            .expect("root replay response length overflow");
        assert_eq!(
            bytes.len(),
            response_end,
            "root replay record response length mismatch"
        );

        Self {
            caller,
            payload_hash,
            issued_at,
            expires_at,
            response_bytes: bytes[response_start..response_end].to_vec(),
        }
    }
}

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

    #[must_use]
    pub(crate) fn active_len_for_caller(caller: Principal, now: u64) -> usize {
        ROOT_REPLAY.with_borrow(|map| {
            map.iter()
                .filter(|entry| entry.value().caller == caller && now <= entry.value().expires_at)
                .count()
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    // round_trip_record
    //
    // Ensure the manual replay record encoding is lossless for stored payloads.
    fn round_trip_record(record: RootReplayRecord) {
        let encoded = record.clone().into_bytes();
        let decoded = RootReplayRecord::from_bytes(Cow::Owned(encoded.clone()));

        assert_eq!(
            decoded, record,
            "manual replay record round-trip must match"
        );
        assert_eq!(
            encoded.len(),
            ROOT_REPLAY_RECORD_MIN_BYTES
                + record.caller.as_slice().len()
                + record.response_bytes.len(),
            "encoded replay record length must match caller, fixed fields, and payload bytes"
        );
    }

    #[test]
    fn root_replay_record_round_trips_empty_response() {
        round_trip_record(RootReplayRecord {
            caller: p(1),
            payload_hash: [7u8; 32],
            issued_at: 11,
            expires_at: 22,
            response_bytes: vec![],
        });
    }

    #[test]
    fn root_replay_record_round_trips_populated_response() {
        round_trip_record(RootReplayRecord {
            caller: p(2),
            payload_hash: [9u8; 32],
            issued_at: 111,
            expires_at: 222,
            response_bytes: vec![1, 2, 3, 4, 5, 6],
        });
    }
}
