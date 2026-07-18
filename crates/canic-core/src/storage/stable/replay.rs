//! Module: storage::stable::replay
//!
//! Responsibility: define stable-memory schemas for replay receipts.
//! Does not own: replay decisions, receipt lifecycle, or command execution.
//! Boundary: storage ops convert between these records and replay model types.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
#[cfg(test)]
use crate::cdk::types::Principal;
use crate::{
    cdk::structures::{DefaultMemoryImpl, Storable, memory::VirtualMemory, storable::Bound},
    eager_static,
    model::replay::{
        CommandKind, ExternalEffectDescriptor, OperationId, REPLAY_RECEIPT_SCHEMA_VERSION,
        ReplayActor, ReplayCostGuardSettlement, ReplayReceipt, ReplayReceiptStatus,
        placement_receipt_requires_acknowledgement,
    },
    role_contract::allocation::memory::auth::REPLAY_RECEIPTS_ID,
    storage::prelude::*,
};
use std::{borrow::Cow, cell::RefCell};

eager_static! {
    static REPLAY_RECEIPTS: RefCell<
        StableBtreeMap<ReplayReceiptSlotKey, ReplayReceiptRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.replay_receipts.v1", ty = ReplayReceiptStore, id = REPLAY_RECEIPTS_ID)),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged_response_schema_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged_response_bytes: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_guard_settlement: Option<ReplayCostGuardSettlement>,
    pub effect: Option<ExternalEffectDescriptor>,
}

impl ReplayReceiptRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "ReplayReceiptRecord";

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
            staged_response_schema_version: receipt.staged_response_schema_version,
            staged_response_bytes: receipt.staged_response_bytes,
            cost_guard_settlement: receipt.cost_guard_settlement,
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
            staged_response_schema_version: self.staged_response_schema_version,
            staged_response_bytes: self.staged_response_bytes,
            cost_guard_settlement: self.cost_guard_settlement,
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
        crate::cdk::serialize::serialize(&self).expect("replay receipt record serializes to cbor")
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        crate::cdk::serialize::deserialize(bytes.as_ref())
            .expect("replay receipt record decodes from cbor")
    }
}

///
/// ReplayReceiptEntryRecord
///
/// One logical replay-receipt snapshot row preserving its stable slot key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayReceiptEntryRecord {
    pub key: ReplayReceiptSlotKey,
    pub record: ReplayReceiptRecord,
}

///
/// ReplayReceiptsData
///
/// Canonical replay-receipt allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReplayReceiptsData {
    pub entries: Vec<ReplayReceiptEntryRecord>,
}

impl ReplayReceiptsData {
    pub const STATE_CONTRACT_NAME: &'static str = "ReplayReceiptsData";
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
                    let record = entry.value();
                    record.actor == actor
                        && (record_survives_replay_expiry(&record)
                            || record
                                .expires_at_ns
                                .is_none_or(|expires_at_ns| now_ns < expires_at_ns))
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

    #[must_use]
    pub(crate) fn has_pending_for_actor_command_excluding_operation(
        actor: ReplayActor,
        command_kind: &str,
        excluded_operation_id: [u8; 32],
        now_ns: u64,
    ) -> bool {
        REPLAY_RECEIPTS.with_borrow(|map| {
            map.iter().any(|entry| {
                let record = entry.value();
                record.actor == actor
                    && record.command_kind == command_kind
                    && record.operation_id != excluded_operation_id
                    && record_is_pending(&record, now_ns)
            })
        })
    }

    pub(crate) fn collect_expired(now_ns: u64, limit: usize) -> Vec<ReplayReceiptSlotKey> {
        let mut expired = Vec::new();
        REPLAY_RECEIPTS.with_borrow(|map| {
            for entry in map.iter() {
                let record = entry.value();
                if !record_survives_replay_expiry(&record)
                    && record
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
    matches!(
        record.status,
        ReplayReceiptStatus::ExternalEffectInFlight | ReplayReceiptStatus::RecoveryRequired { .. }
    ) || (record
        .expires_at_ns
        .is_none_or(|expires_at_ns| now_ns < expires_at_ns)
        && matches!(record.status, ReplayReceiptStatus::Reserved))
}

fn record_survives_replay_expiry(record: &ReplayReceiptRecord) -> bool {
    matches!(
        record.status,
        ReplayReceiptStatus::ExternalEffectInFlight | ReplayReceiptStatus::RecoveryRequired { .. }
    ) || placement_receipt_requires_acknowledgement(&record.status, record.effect.as_ref())
}

#[cfg(test)]
impl ReplayReceiptStore {
    #[must_use]
    pub(crate) fn export() -> ReplayReceiptsData {
        ReplayReceiptsData {
            entries: REPLAY_RECEIPTS.with_borrow(|map| {
                map.iter()
                    .map(|entry| ReplayReceiptEntryRecord {
                        key: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import(data: ReplayReceiptsData) {
        REPLAY_RECEIPTS.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.key, entry.record);
            }
        });
    }

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
            staged_response_schema_version: None,
            staged_response_bytes: None,
            cost_guard_settlement: None,
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
    fn replay_receipt_record_has_exact_stable_bytes() {
        let record = receipt_record_fixture();
        let current_bytes = record.into_bytes();

        assert_eq!(
            current_bytes,
            hex_fixture(
                "ad6e736368656d615f76657273696f6e016c636f6d6d616e645f6b696e646f746573742e636f6d6d616e642e76316c6f7065726174696f6e5f696498200909090909090909090909090909090909090909090909090909090909090909656163746f72a2736566666563746976655f7072696e636970616c581d010101010101010101010101010101010101010101010101010101010169617574685f6b696e646c44697265637443616c6c6572781b7061796c6f61645f686173685f736368656d615f76657273696f6e016c7061796c6f61645f6861736898200707070707070707070707070707070707070707070707070707070707070707667374617475736852657365727665646d637265617465645f61745f6e7318646d757064617465645f61745f6e7318646d657870697265735f61745f6e7318c877726573706f6e73655f736368656d615f76657273696f6ef66e726573706f6e73655f6279746573f666656666656374f6"
            )
        );
    }

    fn hex_fixture(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let pair = std::str::from_utf8(pair).expect("fixture hex is UTF-8");
                u8::from_str_radix(pair, 16).expect("fixture hex byte is valid")
            })
            .collect()
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
    fn staged_cost_recovery_data_survives_stable_round_trip() {
        let mut record = receipt_record_fixture();
        record.status = ReplayReceiptStatus::RecoveryRequired {
            reason: RecoveryReason::CostSettlementFailed,
        };
        record.staged_response_schema_version = Some(1);
        record.staged_response_bytes = Some(vec![1, 2, 3]);
        record.cost_guard_settlement = Some(ReplayCostGuardSettlement {
            quota_intent_id: crate::ids::IntentId(41),
            reservation_intent_id: crate::ids::IntentId(42),
        });

        let receipt = stable_round_trip_into_receipt(record.clone());

        assert_eq!(ReplayReceiptRecord::from_receipt(receipt), record);
    }

    #[test]
    fn external_effect_and_recovery_receipts_are_not_expired_or_removed() {
        ReplayReceiptStore::reset_for_tests();
        let recovery_key = ReplayReceiptSlotKey([44; 32]);
        let mut record = receipt_record_fixture();
        record.status = ReplayReceiptStatus::RecoveryRequired {
            reason: RecoveryReason::CostSettlementFailed,
        };
        let actor = record.actor;
        let command_kind = record.command_kind.clone();
        ReplayReceiptStore::upsert(recovery_key, record);
        let effect_key = ReplayReceiptSlotKey([45; 32]);
        let mut effect = receipt_record_fixture();
        effect.operation_id = [8; 32];
        effect.status = ReplayReceiptStatus::ExternalEffectInFlight;
        ReplayReceiptStore::upsert(effect_key, effect);

        assert_eq!(ReplayReceiptStore::active_len_for_actor(actor, 300), 2);
        assert_eq!(ReplayReceiptStore::pending_len_for_actor(actor, 300), 2);
        assert_eq!(
            ReplayReceiptStore::pending_len_for_command_kind(&command_kind, 300),
            2
        );
        assert!(ReplayReceiptStore::collect_expired(300, 10).is_empty());
        assert!(ReplayReceiptStore::get(recovery_key).is_some());
        assert!(ReplayReceiptStore::get(effect_key).is_some());

        ReplayReceiptStore::reset_for_tests();
    }

    #[test]
    fn pending_actor_command_query_excludes_only_the_current_operation() {
        ReplayReceiptStore::reset_for_tests();
        let key = ReplayReceiptSlotKey([47; 32]);
        let record = receipt_record_fixture();
        let actor = record.actor;
        let command_kind = record.command_kind.clone();
        let operation_id = record.operation_id;
        ReplayReceiptStore::upsert(key, record);

        assert!(
            !ReplayReceiptStore::has_pending_for_actor_command_excluding_operation(
                actor,
                &command_kind,
                operation_id,
                150,
            )
        );
        assert!(
            ReplayReceiptStore::has_pending_for_actor_command_excluding_operation(
                actor,
                &command_kind,
                [8; 32],
                150,
            )
        );
        assert!(
            !ReplayReceiptStore::has_pending_for_actor_command_excluding_operation(
                actor,
                "other.command.v1",
                [8; 32],
                150,
            )
        );
        assert!(
            !ReplayReceiptStore::has_pending_for_actor_command_excluding_operation(
                actor,
                &command_kind,
                [8; 32],
                201,
            )
        );

        ReplayReceiptStore::reset_for_tests();
    }

    #[test]
    fn committed_placement_receipt_survives_expiry_without_counting_as_pending() {
        ReplayReceiptStore::reset_for_tests();
        let key = ReplayReceiptSlotKey([46; 32]);
        let mut record = receipt_record_fixture();
        record.command_kind = crate::model::replay::PLACEMENT_CHILD_REPLAY_COMMAND_KIND.to_string();
        record.status = ReplayReceiptStatus::Committed;
        record.effect = Some(ExternalEffectDescriptor::ManagementCreateCanister {
            command_kind: CommandKind::new(
                crate::model::replay::PLACEMENT_CHILD_REPLAY_COMMAND_KIND,
            )
            .expect("command"),
        });
        let actor = record.actor;
        ReplayReceiptStore::upsert(key, record);

        assert_eq!(ReplayReceiptStore::active_len_for_actor(actor, 300), 1);
        assert_eq!(ReplayReceiptStore::pending_len_for_actor(actor, 300), 0);
        assert!(ReplayReceiptStore::collect_expired(300, 10).is_empty());
        assert!(ReplayReceiptStore::get(key).is_some());

        ReplayReceiptStore::reset_for_tests();
    }

    #[test]
    fn recovery_required_record_has_exact_stable_bytes() {
        let mut record = receipt_record_fixture();
        record.status = ReplayReceiptStatus::RecoveryRequired {
            reason: RecoveryReason::ExternalEffectStatusUnknown,
        };
        record.updated_at_ns = 175;

        assert_eq!(
            record.into_bytes(),
            hex_fixture(
                "ad6e736368656d615f76657273696f6e016c636f6d6d616e645f6b696e646f746573742e636f6d6d616e642e76316c6f7065726174696f6e5f696498200909090909090909090909090909090909090909090909090909090909090909656163746f72a2736566666563746976655f7072696e636970616c581d010101010101010101010101010101010101010101010101010101010169617574685f6b696e646c44697265637443616c6c6572781b7061796c6f61645f686173685f736368656d615f76657273696f6e016c7061796c6f61645f686173689820070707070707070707070707070707070707070707070707070707070707070766737461747573a1705265636f766572795265717569726564a166726561736f6e781b45787465726e616c456666656374537461747573556e6b6e6f776e6d637265617465645f61745f6e7318646d757064617465645f61745f6e7318af6d657870697265735f61745f6e7318c877726573706f6e73655f736368656d615f76657273696f6ef66e726573706f6e73655f6279746573f666656666656374f6"
            )
        );
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

    #[test]
    fn replay_receipts_round_trip_through_canonical_data_snapshot() {
        ReplayReceiptStore::reset_for_tests();
        let key = ReplayReceiptSlotKey([4; 32]);
        ReplayReceiptStore::upsert(key, receipt_record_fixture());

        let data = ReplayReceiptStore::export();
        ReplayReceiptStore::reset_for_tests();
        assert_eq!(ReplayReceiptStore::export(), ReplayReceiptsData::default());

        ReplayReceiptStore::import(data.clone());
        assert_eq!(ReplayReceiptStore::export(), data);
        ReplayReceiptStore::reset_for_tests();
    }
}
