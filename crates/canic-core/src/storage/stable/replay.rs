//! Module: storage::stable::replay
//!
//! Responsibility: define stable-memory schemas for replay receipts.
//! Does not own: replay decisions, receipt lifecycle, or command execution.
//! Boundary: storage ops convert between these records and replay model types.

#[cfg(test)]
use crate::cdk::types::Principal;
use crate::{
    cdk::structures::{DefaultMemoryImpl, Storable, memory::VirtualMemory, storable::Bound},
    eager_static,
    model::replay::{
        CommandKind, ExternalEffectDescriptor, OperationId, REPLAY_RECEIPT_SCHEMA_VERSION,
        ReplayActor, ReplayReceipt, ReplayReceiptStatus,
    },
    storage::{prelude::*, stable::memory::auth::REPLAY_RECEIPTS_ID},
};
use ic_memory::stable_structures::btreemap::BTreeMap as StableBtreeMap;
use std::{borrow::Cow, cell::RefCell};

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
/// Stable key for replay receipt records.
/// Owned by stable storage and derived by replay storage ops.
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
/// Stable-memory representation of a shared replay receipt.
/// Owned by stable storage and converted to `ReplayReceipt` for replay ops.
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
/// ReplayReceiptStore
///
/// Stable BTreeMap facade for shared replay receipt records.
/// Owned by stable storage and wrapped by storage ops.
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

    #[must_use]
    pub(crate) fn pending_len_for_actor(actor: ReplayActor, now_ns: u64) -> usize {
        REPLAY_RECEIPTS.with_borrow(|map| {
            map.iter()
                .filter(|entry| {
                    let record = entry.value();
                    record.actor == actor && record_is_pending(&record, now_ns)
                })
                .count()
        })
    }

    #[must_use]
    pub(crate) fn pending_len_for_command_kind(command_kind: &str, now_ns: u64) -> usize {
        REPLAY_RECEIPTS.with_borrow(|map| {
            map.iter()
                .filter(|entry| {
                    let record = entry.value();
                    record.command_kind == command_kind && record_is_pending(&record, now_ns)
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

fn record_is_pending(record: &ReplayReceiptRecord, now_ns: u64) -> bool {
    record
        .expires_at_ns
        .is_none_or(|expires_at_ns| now_ns < expires_at_ns)
        && matches!(
            record.status,
            ReplayReceiptStatus::Reserved
                | ReplayReceiptStatus::ExternalEffectInFlight
                | ReplayReceiptStatus::RecoveryRequired { .. }
        )
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
    use crate::model::replay::RecoveryReason;

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

    fn stable_round_trip_into_receipt(record: ReplayReceiptRecord) -> ReplayReceipt {
        let encoded = record.into_bytes();
        ReplayReceiptRecord::from_bytes(Cow::Owned(encoded))
            .into_receipt()
            .expect("stable replay receipt decodes")
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
    fn committed_replay_receipt_survives_stable_round_trip() {
        let mut record = receipt_record_fixture();
        record.status = ReplayReceiptStatus::Committed;
        record.updated_at_ns = 150;
        record.response_schema_version = Some(1);
        record.response_bytes = Some(vec![1, 2, 3, 4]);
        record.effect = Some(ExternalEffectDescriptor::ManagementCall {
            canister: p(8),
            method: "deposit_cycles".to_string(),
        });

        let receipt = stable_round_trip_into_receipt(record.clone());

        assert_eq!(receipt.status, ReplayReceiptStatus::Committed);
        assert_eq!(receipt.response_schema_version, Some(1));
        assert_eq!(receipt.response_bytes.as_deref(), Some(&[1, 2, 3, 4][..]));
        assert_eq!(ReplayReceiptRecord::from_receipt(receipt), record);
    }

    #[test]
    fn pending_and_recovery_replay_receipts_survive_stable_round_trip() {
        let reserved = stable_round_trip_into_receipt(receipt_record_fixture());
        assert_eq!(reserved.status, ReplayReceiptStatus::Reserved);
        assert_eq!(reserved.response_bytes, None);
        assert_eq!(reserved.effect, None);

        let effect = ExternalEffectDescriptor::ManagementCall {
            canister: p(8),
            method: "deposit_cycles".to_string(),
        };
        let mut recovery = receipt_record_fixture();
        recovery.status = ReplayReceiptStatus::RecoveryRequired {
            reason: RecoveryReason::ExternalEffectStatusUnknown,
        };
        recovery.updated_at_ns = 175;
        recovery.effect = Some(effect.clone());

        let receipt = stable_round_trip_into_receipt(recovery);

        assert_eq!(
            receipt.status,
            ReplayReceiptStatus::RecoveryRequired {
                reason: RecoveryReason::ExternalEffectStatusUnknown
            }
        );
        assert_eq!(receipt.effect, Some(effect));
    }

    #[test]
    fn unsupported_replay_receipt_schema_returns_controlled_error() {
        let mut record = receipt_record_fixture();
        record.schema_version = REPLAY_RECEIPT_SCHEMA_VERSION + 1;

        record
            .into_receipt()
            .expect_err("unsupported schema must not decode");
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
