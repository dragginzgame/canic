//! Stable-memory intent store primitives.
//!
//! Data-only storage slots for cross-canister intent tracking. The ops layer
//! enforces mechanical invariants (uniqueness, monotonic state transitions,
//! aggregate consistency). Policy and capacity decisions live above this layer.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{
        DefaultMemoryImpl, Storable, cell::Cell, memory::VirtualMemory, storable::Bound,
    },
    ids::{IntentId, IntentResourceKey},
    model::{
        intent::{PayloadBinding, ReceiptBackedIntent, ReceiptBackedIntentState},
        replay::OperationId,
    },
    role_contract::allocation::memory::intent::{
        INTENT_EXPIRY_INDEX_ID, INTENT_META_ID, INTENT_PENDING_ID, INTENT_RECORDS_ID,
        INTENT_TOTALS_ID, PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID, RECEIPT_BACKED_INTENT_RECORDS_ID,
    },
    storage::prelude::*,
};
use std::{borrow::Cow, cell::RefCell};

//
// INTENT STORE
//

pub const INTENT_STORE_SCHEMA_VERSION: u32 = 1;

eager_static! {
    static INTENT_META: RefCell<Cell<IntentStoreMetaRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.intent_meta.v1", ty = IntentStoreMetaRecord, id = INTENT_META_ID),
            IntentStoreMetaRecord::default(),
        ));
}

eager_static! {
    static RECEIPT_BACKED_INTENT_RECORDS: RefCell<
        StableBtreeMap<
            OperationId,
            ReceiptBackedIntentRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >
    > = RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(
        authority = CANIC_CORE_MEMORY_AUTHORITY,
        key = "canic.core.receipt_backed_intent_records.v1",
        ty = ReceiptBackedIntentRecord,
        id = RECEIPT_BACKED_INTENT_RECORDS_ID
    )));
}

eager_static! {
    static INTENT_EXPIRY_INDEX: RefCell<
        StableBtreeMap<IntentExpiryKeyRecord, IntentExpiryEntryRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.intent_expiry_index.v1", ty = IntentExpiryEntryRecord, id = INTENT_EXPIRY_INDEX_ID)),
    );
}

eager_static! {
    static PLACEMENT_ACKNOWLEDGEMENT_INDEX: RefCell<
        StableBtreeMap<
            OperationId,
            PlacementAcknowledgementEntryRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >
    > = RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(
        authority = CANIC_CORE_MEMORY_AUTHORITY,
        key = "canic.core.placement_acknowledgement_index.v1",
        ty = PlacementAcknowledgementEntryRecord,
        id = PLACEMENT_ACKNOWLEDGEMENT_INDEX_ID
    )));
}

eager_static! {
    static INTENT_RECORDS: RefCell<
        StableBtreeMap<IntentId, IntentRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.intent_records.v1", ty = IntentRecord, id = INTENT_RECORDS_ID)),
    );
}

eager_static! {
    static INTENT_TOTALS: RefCell<
        StableBtreeMap<IntentResourceKey, IntentResourceTotalsRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.intent_totals.v1", ty = IntentResourceTotalsRecord, id = INTENT_TOTALS_ID)),
    );
}

eager_static! {
    static INTENT_PENDING: RefCell<
        StableBtreeMap<IntentId, IntentPendingEntryRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        StableBtreeMap::init(crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.intent_pending.v1", ty = IntentPendingEntryRecord, id = INTENT_PENDING_ID)),
    );
}

impl Storable for IntentId {
    const BOUND: Bound = Bound::Bounded {
        max_size: 8,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.0.to_be_bytes().to_vec())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }

    /// Decode the exact fixed-width stable intent identity.
    ///
    /// # Panics
    ///
    /// Panics when stable memory contains an intent ID that is not exactly eight bytes.
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = <[u8; 8]>::try_from(bytes.as_ref()).unwrap_or_else(|_| {
            panic!(
                "stable IntentId is {} bytes; expected 8",
                bytes.as_ref().len()
            )
        });

        Self(u64::from_be_bytes(bytes))
    }
}

impl Storable for OperationId {
    const BOUND: Bound = Bound::Bounded {
        max_size: 32,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.as_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    /// Decode the exact fixed-width stable operation identity.
    ///
    /// # Panics
    ///
    /// Panics when stable memory contains an operation ID that is not exactly 32 bytes.
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let operation_id = <[u8; 32]>::try_from(bytes.as_ref()).unwrap_or_else(|_| {
            panic!(
                "stable OperationId is {} bytes; expected 32",
                bytes.as_ref().len()
            )
        });
        Self::from_bytes(operation_id)
    }
}

/// Ordered stable key for one finite local-intent cleanup deadline.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IntentExpiryKeyRecord {
    pub due_at_secs: u64,
    pub intent_id: IntentId,
}

impl Storable for IntentExpiryKeyRecord {
    const BOUND: Bound = Bound::Bounded {
        max_size: 16,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&self.due_at_secs.to_be_bytes());
        bytes.extend_from_slice(&self.intent_id.0.to_be_bytes());
        bytes
    }

    /// Decode the exact fixed-width stable intent-expiry key.
    ///
    /// # Panics
    ///
    /// Panics when stable memory contains a key that is not exactly sixteen bytes.
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = <[u8; 16]>::try_from(bytes.as_ref()).unwrap_or_else(|_| {
            panic!(
                "stable IntentExpiryKeyRecord is {} bytes; expected 16",
                bytes.as_ref().len()
            )
        });
        let (due_at_secs, intent_id) = bytes.split_at(8);
        Self {
            due_at_secs: u64::from_be_bytes(
                due_at_secs.try_into().expect("expiry key deadline width"),
            ),
            intent_id: IntentId(u64::from_be_bytes(
                intent_id.try_into().expect("expiry key intent width"),
            )),
        }
    }
}

/// Stable value for one finite local-intent cleanup deadline.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntentExpiryEntryRecord {
    pub intent_id: IntentId,
}

impl IntentExpiryEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentExpiryEntryRecord";
    pub const STORABLE_MAX_SIZE: u32 = 32;
}

impl_storable_bounded!(
    IntentExpiryEntryRecord,
    IntentExpiryEntryRecord::STORABLE_MAX_SIZE,
    false
);

///
/// IntentState
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IntentState {
    Pending,
    Committed,
    Aborted,
}

///
/// IntentRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntentRecord {
    pub id: IntentId,
    pub resource_key: IntentResourceKey,
    pub quantity: u64,
    pub state: IntentState,
    pub created_at: u64,
    // TTL is enforced logically at read time; the derived index schedules cleanup.
    pub ttl_secs: Option<u64>,
}

impl IntentRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentRecord";
    pub const STORABLE_MAX_SIZE: u32 = 256;
}

impl_storable_bounded!(IntentRecord, IntentRecord::STORABLE_MAX_SIZE, false);

///
/// IntentStoreMetaRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntentStoreMetaRecord {
    pub schema_version: u32,
    pub next_intent_id: IntentId,
    pub pending_total: u64,
    pub committed_total: u64,
    pub aborted_total: u64,
}

impl IntentStoreMetaRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentStoreMetaRecord";
    pub const STORABLE_MAX_SIZE: u32 = 96;
}

impl Default for IntentStoreMetaRecord {
    fn default() -> Self {
        Self {
            schema_version: INTENT_STORE_SCHEMA_VERSION,
            next_intent_id: IntentId(1),
            pending_total: 0,
            committed_total: 0,
            aborted_total: 0,
        }
    }
}

impl_storable_bounded!(
    IntentStoreMetaRecord,
    IntentStoreMetaRecord::STORABLE_MAX_SIZE,
    false
);

///
/// IntentResourceTotalsRecord
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntentResourceTotalsRecord {
    pub reserved_qty: u64,
    pub committed_qty: u64,
    pub pending_count: u64,
}

impl IntentResourceTotalsRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentResourceTotalsRecord";
    pub const STORABLE_MAX_SIZE: u32 = 64;
}

impl_storable_bounded!(
    IntentResourceTotalsRecord,
    IntentResourceTotalsRecord::STORABLE_MAX_SIZE,
    false
);

///
/// IntentPendingEntryRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntentPendingEntryRecord {
    pub resource_key: IntentResourceKey,
    pub quantity: u64,
    pub created_at: u64,
    // TTL is enforced logically at read time; cleanup is asynchronous.
    pub ttl_secs: Option<u64>,
}

impl IntentPendingEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentPendingEntryRecord";
    pub const STORABLE_MAX_SIZE: u32 = 224;
}

impl_storable_bounded!(
    IntentPendingEntryRecord,
    IntentPendingEntryRecord::STORABLE_MAX_SIZE,
    false
);

/// Stable representation of one durable receipt-backed intent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiptBackedIntentRecord {
    pub schema_version: u32,
    pub operation_id: OperationId,
    pub payload_binding: PayloadBinding,
    pub resource_key: IntentResourceKey,
    pub quantity: u64,
    pub state: ReceiptBackedIntentState,
    pub revision: u64,
    pub created_at_ns: u64,
    pub updated_at_ns: u64,
}

impl ReceiptBackedIntentRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "ReceiptBackedIntentRecord";
    pub const STORABLE_MAX_SIZE: u32 = 1024;

    #[must_use]
    pub fn into_intent(self) -> ReceiptBackedIntent {
        ReceiptBackedIntent {
            schema_version: self.schema_version,
            operation_id: self.operation_id,
            payload_binding: self.payload_binding,
            resource_key: self.resource_key,
            quantity: self.quantity,
            state: self.state,
            revision: self.revision,
            created_at_ns: self.created_at_ns,
            updated_at_ns: self.updated_at_ns,
        }
    }
}

impl_storable_bounded!(
    ReceiptBackedIntentRecord,
    ReceiptBackedIntentRecord::STORABLE_MAX_SIZE,
    false
);

/// Stable value proving that an exact placement operation is queued for root acknowledgement.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlacementAcknowledgementEntryRecord {
    pub operation_id: OperationId,
}

impl PlacementAcknowledgementEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "PlacementAcknowledgementEntryRecord";
    pub const STORABLE_MAX_SIZE: u32 = 128;
}

impl_storable_bounded!(
    PlacementAcknowledgementEntryRecord,
    PlacementAcknowledgementEntryRecord::STORABLE_MAX_SIZE,
    false
);

///
/// IntentMetaData
///
/// Canonical intent-store metadata allocation snapshot.
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IntentMetaData {
    pub record: IntentStoreMetaRecord,
}

impl IntentMetaData {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentMetaData";
}

///
/// IntentRecordEntryRecord
///
/// One logical intent-record snapshot row preserving its stable intent ID key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentRecordEntryRecord {
    pub intent_id: IntentId,
    pub record: IntentRecord,
}

///
/// IntentRecordsData
///
/// Canonical intent-records allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IntentRecordsData {
    pub entries: Vec<IntentRecordEntryRecord>,
}

impl IntentRecordsData {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentRecordsData";
}

///
/// IntentTotalsEntryRecord
///
/// One logical intent-total snapshot row preserving its stable resource key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentTotalsEntryRecord {
    pub resource_key: IntentResourceKey,
    pub record: IntentResourceTotalsRecord,
}

///
/// IntentTotalsData
///
/// Canonical intent-resource-totals allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IntentTotalsData {
    pub entries: Vec<IntentTotalsEntryRecord>,
}

impl IntentTotalsData {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentTotalsData";
}

///
/// IntentPendingIndexEntryRecord
///
/// One logical pending-intent snapshot row preserving its stable intent ID key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentPendingIndexEntryRecord {
    pub intent_id: IntentId,
    pub record: IntentPendingEntryRecord,
}

///
/// IntentPendingData
///
/// Canonical pending-intent allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IntentPendingData {
    pub entries: Vec<IntentPendingIndexEntryRecord>,
}

impl IntentPendingData {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentPendingData";
}

/// One logical finite-expiry snapshot row preserving its stable ordered key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntentExpiryIndexEntryRecord {
    pub key: IntentExpiryKeyRecord,
    pub record: IntentExpiryEntryRecord,
}

/// Canonical finite-expiry index allocation snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IntentExpiryIndexData {
    pub entries: Vec<IntentExpiryIndexEntryRecord>,
}

impl IntentExpiryIndexData {
    pub const STATE_CONTRACT_NAME: &'static str = "IntentExpiryIndexData";
}

/// One logical receipt-backed intent snapshot row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptBackedIntentEntryRecord {
    pub operation_id: OperationId,
    pub record: ReceiptBackedIntentRecord,
}

/// Canonical receipt-backed intent record allocation snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReceiptBackedIntentsData {
    pub entries: Vec<ReceiptBackedIntentEntryRecord>,
}

impl ReceiptBackedIntentsData {
    pub const STATE_CONTRACT_NAME: &'static str = "ReceiptBackedIntentsData";
}

/// One logical placement-acknowledgement index snapshot row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlacementAcknowledgementIndexEntryRecord {
    pub operation_id: OperationId,
    pub record: PlacementAcknowledgementEntryRecord,
}

/// Canonical placement-acknowledgement derived-index allocation snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlacementAcknowledgementIndexData {
    pub entries: Vec<PlacementAcknowledgementIndexEntryRecord>,
}

impl PlacementAcknowledgementIndexData {
    pub const STATE_CONTRACT_NAME: &'static str = "PlacementAcknowledgementIndexData";
}

///
/// IntentStore
///

pub struct IntentStore;

impl IntentStore {
    // -------------------------------------------------------------
    // Meta
    // -------------------------------------------------------------

    #[must_use]
    pub(crate) fn meta() -> IntentStoreMetaRecord {
        INTENT_META.with_borrow(|cell| *cell.get())
    }

    pub(crate) fn set_meta(meta: IntentStoreMetaRecord) {
        INTENT_META.with_borrow_mut(|cell| cell.set(meta));
    }

    // -------------------------------------------------------------
    // Records
    // -------------------------------------------------------------

    #[must_use]
    pub(crate) fn get_record(id: IntentId) -> Option<IntentRecord> {
        INTENT_RECORDS.with_borrow(|map| map.get(&id))
    }

    pub(crate) fn insert_record(record: IntentRecord) -> Option<IntentRecord> {
        INTENT_RECORDS.with_borrow_mut(|map| map.insert(record.id, record))
    }

    // -------------------------------------------------------------
    // Totals
    // -------------------------------------------------------------

    #[must_use]
    pub(crate) fn get_totals(key: &IntentResourceKey) -> Option<IntentResourceTotalsRecord> {
        INTENT_TOTALS.with_borrow(|map| map.get(key))
    }

    pub(crate) fn set_totals(
        key: IntentResourceKey,
        totals: IntentResourceTotalsRecord,
    ) -> Option<IntentResourceTotalsRecord> {
        INTENT_TOTALS.with_borrow_mut(|map| map.insert(key, totals))
    }

    // -------------------------------------------------------------
    // Pending index
    // -------------------------------------------------------------

    #[must_use]
    pub(crate) fn get_pending(id: IntentId) -> Option<IntentPendingEntryRecord> {
        INTENT_PENDING.with_borrow(|map| map.get(&id))
    }

    pub(crate) fn insert_pending(
        id: IntentId,
        entry: IntentPendingEntryRecord,
    ) -> Option<IntentPendingEntryRecord> {
        INTENT_PENDING.with_borrow_mut(|map| map.insert(id, entry))
    }

    pub(crate) fn remove_pending(id: IntentId) -> Option<IntentPendingEntryRecord> {
        INTENT_PENDING.with_borrow_mut(|map| map.remove(&id))
    }

    pub(crate) fn with_pending_entries<R>(
        f: impl FnOnce(
            &StableBtreeMap<IntentId, IntentPendingEntryRecord, VirtualMemory<DefaultMemoryImpl>>,
        ) -> R,
    ) -> R {
        INTENT_PENDING.with_borrow(|map| f(map))
    }

    // -------------------------------------------------------------
    // Finite-expiry derived index
    // -------------------------------------------------------------

    #[must_use]
    pub(crate) fn get_expiry(key: IntentExpiryKeyRecord) -> Option<IntentExpiryEntryRecord> {
        INTENT_EXPIRY_INDEX.with_borrow(|map| map.get(&key))
    }

    pub(crate) fn insert_expiry(
        key: IntentExpiryKeyRecord,
        record: IntentExpiryEntryRecord,
    ) -> Option<IntentExpiryEntryRecord> {
        INTENT_EXPIRY_INDEX.with_borrow_mut(|map| map.insert(key, record))
    }

    pub(crate) fn remove_expiry(key: IntentExpiryKeyRecord) -> Option<IntentExpiryEntryRecord> {
        INTENT_EXPIRY_INDEX.with_borrow_mut(|map| map.remove(&key))
    }

    pub(crate) fn clear_expiry_index() {
        INTENT_EXPIRY_INDEX.with_borrow_mut(StableBtreeMap::clear_new);
    }

    pub(crate) fn with_expiry_entries<R>(
        f: impl FnOnce(
            &StableBtreeMap<
                IntentExpiryKeyRecord,
                IntentExpiryEntryRecord,
                VirtualMemory<DefaultMemoryImpl>,
            >,
        ) -> R,
    ) -> R {
        INTENT_EXPIRY_INDEX.with_borrow(|map| f(map))
    }
}

/// Stable store for receipt-backed operations addressed by exact operation ID.
pub struct ReceiptBackedIntentStore;

impl ReceiptBackedIntentStore {
    #[must_use]
    pub(crate) fn len() -> u64 {
        RECEIPT_BACKED_INTENT_RECORDS.with_borrow(StableBtreeMap::len)
    }

    #[must_use]
    pub(crate) fn get(operation_id: OperationId) -> Option<ReceiptBackedIntentRecord> {
        RECEIPT_BACKED_INTENT_RECORDS.with_borrow(|records| records.get(&operation_id))
    }

    pub(crate) fn insert(record: ReceiptBackedIntentRecord) -> Option<ReceiptBackedIntentRecord> {
        RECEIPT_BACKED_INTENT_RECORDS
            .with_borrow_mut(|records| records.insert(record.operation_id, record))
    }

    pub(crate) fn remove(operation_id: OperationId) -> Option<ReceiptBackedIntentRecord> {
        RECEIPT_BACKED_INTENT_RECORDS.with_borrow_mut(|records| records.remove(&operation_id))
    }

    pub(crate) fn with_records<R>(
        f: impl FnOnce(
            &StableBtreeMap<
                OperationId,
                ReceiptBackedIntentRecord,
                VirtualMemory<DefaultMemoryImpl>,
            >,
        ) -> R,
    ) -> R {
        RECEIPT_BACKED_INTENT_RECORDS.with_borrow(|records| f(records))
    }

    #[must_use]
    pub(crate) fn get_placement_acknowledgement(
        operation_id: OperationId,
    ) -> Option<PlacementAcknowledgementEntryRecord> {
        PLACEMENT_ACKNOWLEDGEMENT_INDEX.with_borrow(|index| index.get(&operation_id))
    }

    pub(crate) fn insert_placement_acknowledgement(
        record: PlacementAcknowledgementEntryRecord,
    ) -> Option<PlacementAcknowledgementEntryRecord> {
        PLACEMENT_ACKNOWLEDGEMENT_INDEX
            .with_borrow_mut(|index| index.insert(record.operation_id, record))
    }

    pub(crate) fn remove_placement_acknowledgement(
        operation_id: OperationId,
    ) -> Option<PlacementAcknowledgementEntryRecord> {
        PLACEMENT_ACKNOWLEDGEMENT_INDEX.with_borrow_mut(|index| index.remove(&operation_id))
    }

    pub(crate) fn clear_placement_acknowledgement_index() {
        PLACEMENT_ACKNOWLEDGEMENT_INDEX.with_borrow_mut(StableBtreeMap::clear_new);
    }

    pub(crate) fn with_placement_acknowledgements<R>(
        f: impl FnOnce(
            &StableBtreeMap<
                OperationId,
                PlacementAcknowledgementEntryRecord,
                VirtualMemory<DefaultMemoryImpl>,
            >,
        ) -> R,
    ) -> R {
        PLACEMENT_ACKNOWLEDGEMENT_INDEX.with_borrow(|index| f(index))
    }
}

//
// ─────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────
//

#[cfg(test)]
impl IntentStore {
    #[must_use]
    pub(crate) fn export_meta() -> IntentMetaData {
        IntentMetaData {
            record: Self::meta(),
        }
    }

    pub(crate) fn import_meta(data: IntentMetaData) {
        Self::set_meta(data.record);
    }

    #[must_use]
    pub(crate) fn export_records() -> IntentRecordsData {
        IntentRecordsData {
            entries: INTENT_RECORDS.with_borrow(|map| {
                map.iter()
                    .map(|entry| IntentRecordEntryRecord {
                        intent_id: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import_records(data: IntentRecordsData) {
        INTENT_RECORDS.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.intent_id, entry.record);
            }
        });
    }

    #[must_use]
    pub(crate) fn export_totals() -> IntentTotalsData {
        IntentTotalsData {
            entries: INTENT_TOTALS.with_borrow(|map| {
                map.iter()
                    .map(|entry| IntentTotalsEntryRecord {
                        resource_key: entry.key().clone(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import_totals(data: IntentTotalsData) {
        INTENT_TOTALS.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.resource_key, entry.record);
            }
        });
    }

    #[must_use]
    pub(crate) fn export_pending() -> IntentPendingData {
        IntentPendingData {
            entries: INTENT_PENDING.with_borrow(|map| {
                map.iter()
                    .map(|entry| IntentPendingIndexEntryRecord {
                        intent_id: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import_pending(data: IntentPendingData) {
        INTENT_PENDING.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.intent_id, entry.record);
            }
        });
    }

    #[must_use]
    pub(crate) fn export_expiry_index() -> IntentExpiryIndexData {
        IntentExpiryIndexData {
            entries: INTENT_EXPIRY_INDEX.with_borrow(|map| {
                map.iter()
                    .map(|entry| IntentExpiryIndexEntryRecord {
                        key: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import_expiry_index(data: IntentExpiryIndexData) {
        INTENT_EXPIRY_INDEX.with_borrow_mut(|map| {
            map.clear_new();
            for entry in data.entries {
                map.insert(entry.key, entry.record);
            }
        });
    }

    pub(crate) fn reset_for_tests() {
        INTENT_RECORDS.with_borrow_mut(StableBtreeMap::clear_new);
        INTENT_TOTALS.with_borrow_mut(StableBtreeMap::clear_new);
        INTENT_PENDING.with_borrow_mut(StableBtreeMap::clear_new);
        INTENT_EXPIRY_INDEX.with_borrow_mut(StableBtreeMap::clear_new);
        INTENT_META.with_borrow_mut(|cell| cell.set(IntentStoreMetaRecord::default()));
        ReceiptBackedIntentStore::reset_for_tests();
    }
}

#[cfg(test)]
impl ReceiptBackedIntentStore {
    #[must_use]
    pub(crate) fn export_records() -> ReceiptBackedIntentsData {
        ReceiptBackedIntentsData {
            entries: RECEIPT_BACKED_INTENT_RECORDS.with_borrow(|records| {
                records
                    .iter()
                    .map(|entry| ReceiptBackedIntentEntryRecord {
                        operation_id: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import_records(data: ReceiptBackedIntentsData) {
        RECEIPT_BACKED_INTENT_RECORDS.with_borrow_mut(|records| {
            records.clear_new();
            for entry in data.entries {
                records.insert(entry.operation_id, entry.record);
            }
        });
    }

    #[must_use]
    pub(crate) fn export_placement_acknowledgement_index() -> PlacementAcknowledgementIndexData {
        PlacementAcknowledgementIndexData {
            entries: PLACEMENT_ACKNOWLEDGEMENT_INDEX.with_borrow(|index| {
                index
                    .iter()
                    .map(|entry| PlacementAcknowledgementIndexEntryRecord {
                        operation_id: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    pub(crate) fn import_placement_acknowledgement_index(data: PlacementAcknowledgementIndexData) {
        PLACEMENT_ACKNOWLEDGEMENT_INDEX.with_borrow_mut(|index| {
            index.clear_new();
            for entry in data.entries {
                index.insert(entry.operation_id, entry.record);
            }
        });
    }

    pub(crate) fn reset_for_tests() {
        RECEIPT_BACKED_INTENT_RECORDS.with_borrow_mut(StableBtreeMap::clear_new);
        PLACEMENT_ACKNOWLEDGEMENT_INDEX.with_borrow_mut(StableBtreeMap::clear_new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        model::intent::{
            RECEIPT_BACKED_INTENT_SCHEMA_VERSION, TerminalEvidence, TerminalEvidenceDecision,
        },
    };

    #[test]
    #[should_panic(expected = "stable IntentId is 7 bytes; expected 8")]
    fn malformed_stable_intent_id_fails_closed() {
        let _ = <IntentId as Storable>::from_bytes(Cow::Owned(vec![0; 7]));
    }

    #[test]
    #[should_panic(expected = "stable OperationId is 31 bytes; expected 32")]
    fn malformed_stable_operation_id_fails_closed() {
        let _ = <OperationId as Storable>::from_bytes(Cow::Owned(vec![0; 31]));
    }

    #[test]
    #[should_panic(expected = "stable IntentExpiryKeyRecord is 15 bytes; expected 16")]
    fn malformed_stable_intent_expiry_key_fails_closed() {
        let _ = <IntentExpiryKeyRecord as Storable>::from_bytes(Cow::Owned(vec![0; 15]));
    }

    #[test]
    fn stable_intent_expiry_key_preserves_deadline_then_identity_order() {
        let keys = [
            IntentExpiryKeyRecord {
                due_at_secs: 11,
                intent_id: IntentId(1),
            },
            IntentExpiryKeyRecord {
                due_at_secs: 10,
                intent_id: IntentId(2),
            },
            IntentExpiryKeyRecord {
                due_at_secs: 10,
                intent_id: IntentId(1),
            },
        ];
        let mut encoded = keys.map(Storable::into_bytes);
        encoded.sort();
        assert_eq!(
            encoded,
            [
                keys[2].into_bytes(),
                keys[1].into_bytes(),
                keys[0].into_bytes()
            ]
        );
    }

    #[test]
    fn intent_allocations_round_trip_through_canonical_data_snapshots() {
        IntentStore::reset_for_tests();
        let intent_id = IntentId(7);
        let resource_key = IntentResourceKey::new("storage:uploads");
        let record = IntentRecord {
            id: intent_id,
            resource_key: resource_key.clone(),
            quantity: 11,
            state: IntentState::Pending,
            created_at: 13,
            ttl_secs: Some(17),
        };
        let totals = IntentResourceTotalsRecord {
            reserved_qty: 11,
            committed_qty: 19,
            pending_count: 1,
        };
        let pending = IntentPendingEntryRecord {
            resource_key: resource_key.clone(),
            quantity: 11,
            created_at: 13,
            ttl_secs: Some(17),
        };
        let meta = IntentStoreMetaRecord {
            schema_version: INTENT_STORE_SCHEMA_VERSION,
            next_intent_id: IntentId(8),
            pending_total: 1,
            committed_total: 2,
            aborted_total: 3,
        };

        IntentStore::set_meta(meta);
        IntentStore::insert_record(record);
        IntentStore::set_totals(resource_key, totals);
        IntentStore::insert_pending(intent_id, pending);
        let expiry_key = IntentExpiryKeyRecord {
            due_at_secs: 31,
            intent_id,
        };
        IntentStore::insert_expiry(expiry_key, IntentExpiryEntryRecord { intent_id });

        let meta_data = IntentStore::export_meta();
        let records_data = IntentStore::export_records();
        let totals_data = IntentStore::export_totals();
        let pending_data = IntentStore::export_pending();
        let expiry_data = IntentStore::export_expiry_index();

        IntentStore::reset_for_tests();
        IntentStore::import_meta(meta_data);
        IntentStore::import_records(records_data.clone());
        IntentStore::import_totals(totals_data.clone());
        IntentStore::import_pending(pending_data.clone());
        IntentStore::import_expiry_index(expiry_data.clone());

        assert_eq!(IntentStore::export_meta(), meta_data);
        assert_eq!(IntentStore::export_records(), records_data);
        assert_eq!(IntentStore::export_totals(), totals_data);
        assert_eq!(IntentStore::export_pending(), pending_data);
        assert_eq!(IntentStore::export_expiry_index(), expiry_data);
        IntentStore::reset_for_tests();
    }

    #[test]
    fn receipt_backed_allocations_round_trip_through_canonical_data_snapshots() {
        IntentStore::reset_for_tests();
        let operation_id = OperationId::from_bytes([7; 32]);
        let evidence = TerminalEvidence::new(
            Principal::from_slice(&[1; 29]),
            TerminalEvidenceDecision::Committed,
            [8; 32],
        );
        let record = ReceiptBackedIntentRecord {
            schema_version: RECEIPT_BACKED_INTENT_SCHEMA_VERSION,
            operation_id,
            payload_binding: PayloadBinding::new([9; 32]),
            resource_key: IntentResourceKey::new("mint:collection"),
            quantity: 11,
            state: ReceiptBackedIntentState::Committed { evidence },
            revision: 2,
            created_at_ns: 13,
            updated_at_ns: 17,
        };
        ReceiptBackedIntentStore::insert(record);
        ReceiptBackedIntentStore::insert_placement_acknowledgement(
            PlacementAcknowledgementEntryRecord { operation_id },
        );
        let records_data = ReceiptBackedIntentStore::export_records();
        let acknowledgement_data =
            ReceiptBackedIntentStore::export_placement_acknowledgement_index();

        ReceiptBackedIntentStore::reset_for_tests();
        assert_eq!(
            ReceiptBackedIntentStore::export_records(),
            ReceiptBackedIntentsData::default()
        );
        assert_eq!(
            ReceiptBackedIntentStore::export_placement_acknowledgement_index(),
            PlacementAcknowledgementIndexData::default()
        );

        ReceiptBackedIntentStore::import_records(records_data.clone());
        ReceiptBackedIntentStore::import_placement_acknowledgement_index(
            acknowledgement_data.clone(),
        );
        assert_eq!(ReceiptBackedIntentStore::len(), 1);
        assert_eq!(ReceiptBackedIntentStore::export_records(), records_data);
        assert_eq!(
            ReceiptBackedIntentStore::export_placement_acknowledgement_index(),
            acknowledgement_data
        );
        IntentStore::reset_for_tests();
    }
}
