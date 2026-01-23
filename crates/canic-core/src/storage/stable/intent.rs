//! Stable-memory intent store primitives.
//!
//! Data-only storage slots for cross-canister intent tracking. The ops layer
//! enforces mechanical invariants (uniqueness, monotonic state transitions,
//! aggregate consistency). Policy and capacity decisions live above this layer.

use crate::{
    cdk::structures::{
        BTreeMap, DefaultMemoryImpl, Storable, cell::Cell, memory::VirtualMemory, storable::Bound,
    },
    ids::IntentResourceKey,
    storage::{
        prelude::*,
        stable::memory::intent::{
            INTENT_META_ID, INTENT_PENDING_ID, INTENT_RECORDS_ID, INTENT_TOTALS_ID,
        },
    },
};
use std::{
    borrow::Cow,
    cell::RefCell,
    fmt::{self, Display},
};

//
// INTENT STORE
//

pub const INTENT_STORE_SCHEMA_VERSION: u32 = 1;

eager_static! {
    static INTENT_META: RefCell<Cell<IntentStoreMetaRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(IntentStoreMetaRecord, INTENT_META_ID),
            IntentStoreMetaRecord::default(),
        ));
}

eager_static! {
    static INTENT_RECORDS: RefCell<
        BTreeMap<IntentId, IntentRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(IntentRecord, INTENT_RECORDS_ID)),
    );
}

eager_static! {
    static INTENT_TOTALS: RefCell<
        BTreeMap<IntentResourceKey, IntentResourceTotalsRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(IntentResourceTotalsRecord, INTENT_TOTALS_ID)),
    );
}

eager_static! {
    static INTENT_PENDING: RefCell<
        BTreeMap<IntentId, IntentPendingEntryRecord, VirtualMemory<DefaultMemoryImpl>>
    > = RefCell::new(
        BTreeMap::init(ic_memory!(IntentPendingEntryRecord, INTENT_PENDING_ID)),
    );
}

///
/// IntentId
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct IntentId(pub u64);

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

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let b = bytes.as_ref();

        if b.len() != 8 {
            return Self::default();
        }

        let mut arr = [0u8; 8];
        arr.copy_from_slice(b);

        Self(u64::from_be_bytes(arr))
    }
}

impl Display for IntentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
    // TTL is enforced logically at read time; cleanup is asynchronous.
    pub ttl_secs: Option<u64>,
}

impl IntentRecord {
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

impl IntentStoreMetaRecord {
    pub const STORABLE_MAX_SIZE: u32 = 96;
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
    pub const STORABLE_MAX_SIZE: u32 = 224;
}

impl_storable_bounded!(
    IntentPendingEntryRecord,
    IntentPendingEntryRecord::STORABLE_MAX_SIZE,
    false
);

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
            &BTreeMap<IntentId, IntentPendingEntryRecord, VirtualMemory<DefaultMemoryImpl>>,
        ) -> R,
    ) -> R {
        INTENT_PENDING.with_borrow(|map| f(map))
    }
}

//
// ─────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────
//

#[cfg(test)]
impl IntentStore {
    pub(crate) fn reset_for_tests() {
        INTENT_RECORDS.with_borrow_mut(BTreeMap::clear);
        INTENT_TOTALS.with_borrow_mut(BTreeMap::clear);
        INTENT_PENDING.with_borrow_mut(BTreeMap::clear);
        INTENT_META.with_borrow_mut(|cell| cell.set(IntentStoreMetaRecord::default()));
    }
}
