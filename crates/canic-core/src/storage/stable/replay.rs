use crate::{
    cdk::structures::{
        BTreeMap, DefaultMemoryImpl, Storable, memory::VirtualMemory, storable::Bound,
    },
    eager_static, ic_memory,
    storage::{prelude::*, stable::memory::auth::ROOT_REPLAY_ID},
};
use std::{borrow::Cow, cell::RefCell};

const ROOT_REPLAY_RECORD_FIXED_BYTES: usize = 52;

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
    pub response_bytes: Vec<u8>,
}

impl Storable for RootReplayRecord {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.clone().into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let response_len =
            u32::try_from(self.response_bytes.len()).expect("root replay response bytes fit u32");
        let mut bytes = Vec::with_capacity(
            ROOT_REPLAY_RECORD_FIXED_BYTES + usize::try_from(response_len).unwrap_or(usize::MAX),
        );
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
            bytes.len() >= ROOT_REPLAY_RECORD_FIXED_BYTES,
            "root replay record shorter than fixed header"
        );

        let mut payload_hash = [0u8; 32];
        payload_hash.copy_from_slice(&bytes[0..32]);

        let issued_at = u64::from_le_bytes(
            bytes[32..40]
                .try_into()
                .expect("root replay record issued_at"),
        );
        let expires_at = u64::from_le_bytes(
            bytes[40..48]
                .try_into()
                .expect("root replay record expires_at"),
        );
        let response_len = u32::from_le_bytes(
            bytes[48..52]
                .try_into()
                .expect("root replay record response length"),
        ) as usize;
        let response_end = ROOT_REPLAY_RECORD_FIXED_BYTES
            .checked_add(response_len)
            .expect("root replay response length overflow");
        assert_eq!(
            bytes.len(),
            response_end,
            "root replay record response length mismatch"
        );

        Self {
            payload_hash,
            issued_at,
            expires_at,
            response_bytes: bytes[ROOT_REPLAY_RECORD_FIXED_BYTES..response_end].to_vec(),
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
            ROOT_REPLAY_RECORD_FIXED_BYTES + record.response_bytes.len(),
            "encoded replay record length must match fixed header plus payload bytes"
        );
    }

    #[test]
    fn root_replay_record_round_trips_empty_response() {
        round_trip_record(RootReplayRecord {
            payload_hash: [7u8; 32],
            issued_at: 11,
            expires_at: 22,
            response_bytes: vec![],
        });
    }

    #[test]
    fn root_replay_record_round_trips_populated_response() {
        round_trip_record(RootReplayRecord {
            payload_hash: [9u8; 32],
            issued_at: 111,
            expires_at: 222,
            response_bytes: vec![1, 2, 3, 4, 5, 6],
        });
    }
}
