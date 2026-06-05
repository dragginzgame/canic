#[cfg(test)]
use crate::cdk::types::Principal;
use crate::{
    cdk::structures::{DefaultMemoryImpl, Storable, memory::VirtualMemory, storable::Bound},
    eager_static,
    ops::replay::model::{
        CommandKind, ExternalEffectDescriptor, OperationId, REPLAY_RECEIPT_SCHEMA_VERSION,
        ReplayActor, ReplayReceipt, ReplayReceiptStatus,
    },
    storage::{prelude::*, stable::memory::auth::REPLAY_RECEIPTS_ID},
};
use ic_memory::stable_structures::btreemap::BTreeMap as StableBtreeMap;
use std::{borrow::Cow, cell::RefCell};

#[cfg(test)]
const ROOT_REPLAY_RECORD_MIN_BYTES: usize = 1 + 32 + 8 + 8 + 4;

eager_static! {
    static REPLAY_RECEIPTS: RefCell<
        StableBtreeMap<ReplayReceiptSlotKey, ReplayReceiptRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!("canic.core.replay_receipts.v1", ReplayReceiptStore, REPLAY_RECEIPTS_ID)),
    );
}

///
/// ReplayReceiptSlotKey
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ReplayReceiptSlotKey(pub [u8; 32]);

impl Storable for ReplayReceiptSlotKey {
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
/// ReplayReceiptRecord
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReplayReceiptRecord {
    pub schema_version: u32,
    pub command_kind: String,
    pub operation_id: [u8; 32],
    pub actor: ReplayActor,
    pub payload_hash_schema_version: u32,
    pub payload_hash: [u8; 32],
    pub status: ReplayReceiptStatus,
    pub created_at_ns: u64,
    pub updated_at_ns: u64,
    #[serde(default)]
    pub expires_at_ns: Option<u64>,
    pub response_schema_version: Option<u32>,
    pub response_bytes: Option<Vec<u8>>,
    pub effect: Option<ExternalEffectDescriptor>,
}

impl ReplayReceiptRecord {
    pub fn from_receipt(receipt: ReplayReceipt) -> Self {
        Self {
            schema_version: receipt.schema_version,
            command_kind: receipt.command_kind.as_str().to_string(),
            operation_id: receipt.operation_id.into_bytes(),
            actor: receipt.actor,
            payload_hash_schema_version: receipt.payload_hash_schema_version,
            payload_hash: receipt.payload_hash,
            status: receipt.status,
            created_at_ns: receipt.created_at_ns,
            updated_at_ns: receipt.updated_at_ns,
            expires_at_ns: receipt.expires_at_ns,
            response_schema_version: receipt.response_schema_version,
            response_bytes: receipt.response_bytes,
            effect: receipt.effect,
        }
    }

    pub fn into_receipt(self) -> Result<ReplayReceipt, String> {
        if self.schema_version != REPLAY_RECEIPT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported replay receipt schema version {}",
                self.schema_version
            ));
        }
        Ok(ReplayReceipt {
            schema_version: self.schema_version,
            command_kind: CommandKind::new(self.command_kind)
                .map_err(|err| format!("invalid replay receipt command kind: {err:?}"))?,
            operation_id: OperationId::from_bytes(self.operation_id),
            actor: self.actor,
            payload_hash_schema_version: self.payload_hash_schema_version,
            payload_hash: self.payload_hash,
            status: self.status,
            created_at_ns: self.created_at_ns,
            updated_at_ns: self.updated_at_ns,
            expires_at_ns: self.expires_at_ns,
            response_schema_version: self.response_schema_version,
            response_bytes: self.response_bytes,
            effect: self.effect,
        })
    }
}

impl Storable for ReplayReceiptRecord {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.clone().into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        serde_cbor::to_vec(&self).expect("replay receipt record serializes to cbor")
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        serde_cbor::from_slice(bytes.as_ref()).expect("replay receipt record decodes from cbor")
    }
}

///
/// RootReplayRecord
///

#[cfg(test)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootReplayRecord {
    pub caller: Principal,
    pub payload_hash: [u8; 32],
    pub issued_at: u64,
    pub expires_at: u64,
    pub response_bytes: Vec<u8>,
}

#[cfg(test)]
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
/// ReplayReceiptStore
///
pub struct ReplayReceiptStore;

impl ReplayReceiptStore {
    #[must_use]
    pub(crate) fn get(key: ReplayReceiptSlotKey) -> Option<ReplayReceiptRecord> {
        REPLAY_RECEIPTS.with_borrow(|map| map.get(&key))
    }

    #[must_use]
    pub(crate) fn list_by_actor_operation_excluding_command(
        actor: ReplayActor,
        operation_id: [u8; 32],
        command_kind: &str,
    ) -> Vec<ReplayReceiptRecord> {
        REPLAY_RECEIPTS.with_borrow(|map| {
            map.iter()
                .map(|entry| entry.value())
                .filter(|record| {
                    record.actor == actor
                        && record.operation_id == operation_id
                        && record.command_kind != command_kind
                })
                .collect()
        })
    }

    pub(crate) fn upsert(key: ReplayReceiptSlotKey, record: ReplayReceiptRecord) {
        REPLAY_RECEIPTS.with_borrow_mut(|map| {
            map.insert(key, record);
        });
    }

    pub(crate) fn remove(key: ReplayReceiptSlotKey) -> Option<ReplayReceiptRecord> {
        REPLAY_RECEIPTS.with_borrow_mut(|map| map.remove(&key))
    }

    #[must_use]
    pub(crate) fn len() -> usize {
        REPLAY_RECEIPTS.with_borrow(|map| usize::try_from(map.len()).unwrap_or(usize::MAX))
    }

    #[must_use]
    pub(crate) fn active_len_for_actor(actor: ReplayActor, now_ns: u64) -> usize {
        REPLAY_RECEIPTS.with_borrow(|map| {
            map.iter()
                .filter(|entry| {
                    entry.value().actor == actor
                        && entry
                            .value()
                            .expires_at_ns
                            .is_none_or(|expires_at_ns| now_ns <= expires_at_ns)
                })
                .count()
        })
    }

    pub(crate) fn collect_expired(now_ns: u64, limit: usize) -> Vec<ReplayReceiptSlotKey> {
        let mut expired = Vec::new();
        REPLAY_RECEIPTS.with_borrow(|map| {
            for entry in map.iter() {
                if entry
                    .value()
                    .expires_at_ns
                    .is_some_and(|expires_at_ns| expires_at_ns < now_ns)
                {
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
impl ReplayReceiptStore {
    pub(crate) fn reset_for_tests() {
        REPLAY_RECEIPTS.with_borrow_mut(StableBtreeMap::clear_new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn receipt_record_fixture() -> ReplayReceiptRecord {
        ReplayReceiptRecord {
            schema_version: REPLAY_RECEIPT_SCHEMA_VERSION,
            command_kind: "test.command.v1".to_string(),
            operation_id: [9; 32],
            actor: ReplayActor::direct_caller(p(1)),
            payload_hash_schema_version: 1,
            payload_hash: [7; 32],
            status: ReplayReceiptStatus::Reserved,
            created_at_ns: 100,
            updated_at_ns: 100,
            expires_at_ns: Some(200),
            response_schema_version: None,
            response_bytes: None,
            effect: None,
        }
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

    #[test]
    fn replay_receipt_record_round_trips_through_cbor_storable() {
        let record = receipt_record_fixture();
        let encoded = record.clone().into_bytes();
        let decoded = ReplayReceiptRecord::from_bytes(Cow::Owned(encoded));

        assert_eq!(decoded, record);
    }

    #[test]
    fn replay_receipt_record_converts_to_shared_receipt() {
        let record = receipt_record_fixture();
        let receipt = record.clone().into_receipt().expect("receipt");
        let round_trip = ReplayReceiptRecord::from_receipt(receipt);

        assert_eq!(round_trip, record);
    }

    #[test]
    fn replay_receipt_store_lists_actor_operation_matches_excluding_command() {
        ReplayReceiptStore::reset_for_tests();
        let actor = ReplayActor::direct_caller(p(1));
        let mut a = receipt_record_fixture();
        a.command_kind = "root.upgrade.v1".to_string();
        a.actor = actor;
        a.operation_id = [8; 32];
        let mut b = a.clone();
        b.command_kind = "root.request_cycles.v1".to_string();
        let mut other_actor = a.clone();
        other_actor.actor = ReplayActor::direct_caller(p(2));

        ReplayReceiptStore::upsert(ReplayReceiptSlotKey([1; 32]), a);
        ReplayReceiptStore::upsert(ReplayReceiptSlotKey([2; 32]), b);
        ReplayReceiptStore::upsert(ReplayReceiptSlotKey([3; 32]), other_actor);

        let matches = ReplayReceiptStore::list_by_actor_operation_excluding_command(
            actor,
            [8; 32],
            "root.request_cycles.v1",
        );

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].command_kind, "root.upgrade.v1");

        ReplayReceiptStore::reset_for_tests();
    }
}
