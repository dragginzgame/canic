//! Module: storage::stable::cycles
//!
//! Responsibility: define stable-memory schemas for cycle telemetry.
//! Does not own: cycle funding policy, DTO mapping, or runtime metrics.
//! Boundary: storage ops wrap these records before workflow access.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, Storable, memory::VirtualMemory, storable::Bound},
    eager_static,
    role_contract::allocation::memory::observability::{
        CYCLE_TOPUP_EVENTS_ID, CYCLE_TRACKER_ID, CYCLES_FUNDING_LEDGER_ID,
    },
    storage::prelude::*,
};
use std::{borrow::Cow, cell::RefCell};

eager_static! {
    //
    // CYCLE_TRACKER
    //
    static CYCLE_TRACKER: RefCell<CycleTracker> =
        RefCell::new(CycleTracker::new(StableBtreeMap::init(
            crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.cycle_tracker.v1", ty = CycleTracker, id = CYCLE_TRACKER_ID),
        )));
}

eager_static! {
    //
    // CYCLE_TOPUP_EVENTS
    //
    static CYCLE_TOPUP_EVENTS: RefCell<CycleTopupEvents> =
        RefCell::new(CycleTopupEvents::new(StableBtreeMap::init(
            crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.cycle_topup_events.v1", ty = CycleTopupEvents, id = CYCLE_TOPUP_EVENTS_ID),
        )));
}

eager_static! {
    //
    // CYCLES_FUNDING_LEDGER
    //
    static CYCLES_FUNDING_LEDGER: RefCell<CyclesFundingLedger> =
        RefCell::new(CyclesFundingLedger::new(StableBtreeMap::init(
            crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.cycles_funding_ledger.v1", ty = CyclesFundingLedger, id = CYCLES_FUNDING_LEDGER_ID),
        )));
}

///
/// CycleTracker
///
/// Stable map of observed cycle balances by timestamp.
/// Owned by stable storage and wrapped by cycle storage ops.
///

pub struct CycleTracker {
    map: StableBtreeMap<u64, Cycles, VirtualMemory<DefaultMemoryImpl>>,
}

///
/// CyclesFundingLedgerRecord
///
/// Stable record for per-child funding budget and cooldown state.
/// Owned by stable storage and projected through runtime funding ops.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CyclesFundingLedgerRecord {
    pub granted_total: Cycles,
    pub last_granted_at: u64,
}

impl CyclesFundingLedgerRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CyclesFundingLedgerRecord";
    pub const STORABLE_MAX_SIZE: u32 = 64;
}

impl_storable_bounded!(
    CyclesFundingLedgerRecord,
    CyclesFundingLedgerRecord::STORABLE_MAX_SIZE,
    false
);

///
/// CyclesFundingLedgerEntryRecord
///
/// One logical funding-ledger snapshot row preserving its child principal key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CyclesFundingLedgerEntryRecord {
    pub child: Principal,
    pub record: CyclesFundingLedgerRecord,
}

///
/// CyclesFundingLedgerData
///
/// Canonical cycles-funding-ledger allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CyclesFundingLedgerData {
    pub entries: Vec<CyclesFundingLedgerEntryRecord>,
}

impl CyclesFundingLedgerData {
    pub const STATE_CONTRACT_NAME: &'static str = "CyclesFundingLedgerData";
}

///
/// CyclesFundingLedger
///
/// Stable map facade for child funding budget and cooldown accounting.
/// Owned by stable storage and wrapped by cycle storage ops.
///

pub struct CyclesFundingLedger {
    map: StableBtreeMap<Principal, CyclesFundingLedgerRecord, VirtualMemory<DefaultMemoryImpl>>,
}

impl CyclesFundingLedger {
    pub const fn new(
        map: StableBtreeMap<Principal, CyclesFundingLedgerRecord, VirtualMemory<DefaultMemoryImpl>>,
    ) -> Self {
        Self { map }
    }

    #[must_use]
    pub(crate) fn snapshot(child: Principal) -> Option<CyclesFundingLedgerRecord> {
        CYCLES_FUNDING_LEDGER.with_borrow(|ledger| ledger.map.get(&child))
    }

    pub(crate) fn record_child_grant(child: Principal, granted_cycles: Cycles, now_secs: u64) {
        CYCLES_FUNDING_LEDGER.with_borrow_mut(|ledger| {
            let mut record = ledger.map.get(&child).unwrap_or_default();
            let granted_total = record
                .granted_total
                .to_u128()
                .saturating_add(granted_cycles.to_u128());
            record.granted_total = Cycles::new(granted_total);
            record.last_granted_at = now_secs;
            ledger.map.insert(child, record);
        });
    }

    pub(crate) fn set_snapshot(child: Principal, record: CyclesFundingLedgerRecord) {
        CYCLES_FUNDING_LEDGER.with_borrow_mut(|ledger| {
            ledger.map.insert(child, record);
        });
    }

    #[cfg(test)]
    pub(crate) fn clear_for_tests() {
        CYCLES_FUNDING_LEDGER.with_borrow_mut(|ledger| ledger.map.clear_new());
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn export() -> CyclesFundingLedgerData {
        CyclesFundingLedgerData {
            entries: CYCLES_FUNDING_LEDGER.with_borrow(|ledger| {
                ledger
                    .map
                    .iter()
                    .map(|entry| CyclesFundingLedgerEntryRecord {
                        child: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: CyclesFundingLedgerData) {
        CYCLES_FUNDING_LEDGER.with_borrow_mut(|ledger| {
            ledger.map.clear_new();
            for entry in data.entries {
                ledger.map.insert(entry.child, entry.record);
            }
        });
    }
}

///
/// CycleTopupEventKey
///
/// Stable key for ordered cycle top-up event history.
/// Owned by stable storage and encoded as timestamp plus sequence.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CycleTopupEventKey {
    pub timestamp_secs: u64,
    pub sequence: u32,
}

impl CycleTopupEventKey {
    const STORABLE_SIZE: u32 = 12;

    const fn new(timestamp_secs: u64, sequence: u32) -> Self {
        Self {
            timestamp_secs,
            sequence,
        }
    }
}

impl Storable for CycleTopupEventKey {
    const BOUND: Bound = Bound::Bounded {
        max_size: Self::STORABLE_SIZE,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.into_bytes())
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::STORABLE_SIZE as usize);
        bytes.extend_from_slice(&self.timestamp_secs.to_be_bytes());
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let bytes = bytes.as_ref();
        assert!(
            bytes.len() == Self::STORABLE_SIZE as usize,
            "cycle topup event key has unexpected length"
        );
        let timestamp_secs =
            u64::from_be_bytes(bytes[0..8].try_into().expect("cycle topup timestamp bytes"));
        let sequence =
            u32::from_be_bytes(bytes[8..12].try_into().expect("cycle topup sequence bytes"));

        Self::new(timestamp_secs, sequence)
    }
}

///
/// CycleTopupEventStatusRecord
///
/// Stable status for a cycle top-up event.
/// Owned by stable storage and converted to the boundary DTO enum by storage ops.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum CycleTopupEventStatusRecord {
    RequestErr,
    RequestOk,
    RequestScheduled,
}

///
/// CycleTopupEventRecord
///
/// Stable record for one cycle top-up event.
/// Owned by stable storage and projected through cycle storage ops.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CycleTopupEventRecord {
    pub requested_cycles: Cycles,
    pub transferred_cycles: Option<Cycles>,
    pub status: CycleTopupEventStatusRecord,
    pub error: Option<String>,
}

impl CycleTopupEventRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "CycleTopupEventRecord";
    pub const STORABLE_MAX_SIZE: u32 = 512;
}

impl_storable_bounded!(
    CycleTopupEventRecord,
    CycleTopupEventRecord::STORABLE_MAX_SIZE,
    false
);

///
/// CycleTopupEventEntryRecord
///
/// One logical cycle-top-up snapshot row preserving its stable event key.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CycleTopupEventEntryRecord {
    pub key: CycleTopupEventKey,
    pub record: CycleTopupEventRecord,
}

///
/// CycleTopupEventsData
///
/// Canonical cycle-top-up-event allocation snapshot.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CycleTopupEventsData {
    pub entries: Vec<CycleTopupEventEntryRecord>,
}

impl CycleTopupEventsData {
    pub const STATE_CONTRACT_NAME: &'static str = "CycleTopupEventsData";
}

///
/// CycleTopupEvents
///
/// Stable map facade for cycle top-up event records.
/// Owned by stable storage and wrapped by cycle storage ops.
///

pub struct CycleTopupEvents {
    map:
        StableBtreeMap<CycleTopupEventKey, CycleTopupEventRecord, VirtualMemory<DefaultMemoryImpl>>,
}

impl CycleTopupEvents {
    pub const fn new(
        map: StableBtreeMap<
            CycleTopupEventKey,
            CycleTopupEventRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >,
    ) -> Self {
        Self { map }
    }

    pub(crate) fn record(
        timestamp_secs: u64,
        requested_cycles: Cycles,
        transferred_cycles: Option<Cycles>,
        status: CycleTopupEventStatusRecord,
        error: Option<String>,
    ) {
        CYCLE_TOPUP_EVENTS.with_borrow_mut(|events| {
            let sequence = events.next_sequence(timestamp_secs);
            events.map.insert(
                CycleTopupEventKey::new(timestamp_secs, sequence),
                CycleTopupEventRecord {
                    requested_cycles,
                    transferred_cycles,
                    status,
                    error,
                },
            );
        });
    }

    #[must_use]
    pub(crate) fn purge_before(cutoff: u64, limit: usize) -> usize {
        CYCLE_TOPUP_EVENTS.with_borrow_mut(|events| events.purge_inner(cutoff, limit))
    }

    #[must_use]
    pub(crate) fn data(offset: usize, limit: usize) -> CycleTopupEventsData {
        CycleTopupEventsData {
            entries: CYCLE_TOPUP_EVENTS.with_borrow(|events| {
                events
                    .map
                    .iter()
                    .skip(offset)
                    .take(limit)
                    .map(|entry| CycleTopupEventEntryRecord {
                        key: *entry.key(),
                        record: entry.value(),
                    })
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: CycleTopupEventsData) {
        CYCLE_TOPUP_EVENTS.with_borrow_mut(|events| {
            events.map.clear_new();
            for entry in data.entries {
                events.map.insert(entry.key, entry.record);
            }
        });
    }

    #[cfg(test)]
    pub(crate) fn clear_for_tests() {
        CYCLE_TOPUP_EVENTS.with_borrow_mut(|events| events.map.clear_new());
    }

    fn purge_inner(&mut self, cutoff: u64, limit: usize) -> usize {
        let mut purged = 0;

        while purged < limit
            && let Some((first_key, _)) = self.map.first_key_value()
        {
            if first_key.timestamp_secs < cutoff {
                self.map.remove(&first_key);
                purged += 1;
            } else {
                break;
            }
        }

        purged
    }

    fn next_sequence(&self, timestamp_secs: u64) -> u32 {
        self.map
            .iter()
            .filter(|entry| entry.key().timestamp_secs == timestamp_secs)
            .map(|entry| entry.key().sequence)
            .max()
            .map_or(0, |sequence| sequence.saturating_add(1))
    }
}

impl CycleTracker {
    pub const fn new(map: StableBtreeMap<u64, Cycles, VirtualMemory<DefaultMemoryImpl>>) -> Self {
        Self { map }
    }

    pub(crate) fn record(now: u64, cycles: Cycles) {
        CYCLE_TRACKER.with_borrow_mut(|t| t.insert(now, cycles));
    }

    /// Purge entries older than the provided cutoff timestamp.
    #[must_use]
    pub(crate) fn purge_before(cutoff: u64, limit: usize) -> usize {
        CYCLE_TRACKER.with_borrow_mut(|t| t.purge_inner(cutoff, limit))
    }

    #[must_use]
    pub(crate) fn latest() -> Option<(u64, Cycles)> {
        CYCLE_TRACKER.with_borrow(|tracker| tracker.map.last_key_value())
    }

    #[must_use]
    pub(crate) fn entries(offset: usize, limit: usize) -> Vec<(u64, Cycles)> {
        CYCLE_TRACKER.with_borrow(|t| {
            t.map
                .iter()
                .skip(offset)
                .take(limit)
                .map(|entry| (*entry.key(), entry.value()))
                .collect()
        })
    }

    fn purge_inner(&mut self, cutoff: u64, limit: usize) -> usize {
        let mut purged = 0;

        while purged < limit
            && let Some((first_ts, _)) = self.map.first_key_value()
        {
            if first_ts < cutoff {
                self.map.remove(&first_ts);
                purged += 1;
            } else {
                break;
            }
        }

        purged
    }

    fn insert(&mut self, now: u64, cycles: Cycles) -> bool {
        self.map.insert(now, cycles).is_some()
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::seams;

    #[test]
    fn cycle_history_round_trips_through_canonical_data_snapshots() {
        let _guard = seams::lock();
        CycleTopupEvents::clear_for_tests();
        CyclesFundingLedger::clear_for_tests();

        let child = seams::p(17);
        CycleTopupEvents::record(
            10,
            Cycles::new(20),
            Some(Cycles::new(19)),
            CycleTopupEventStatusRecord::RequestOk,
            None,
        );
        CyclesFundingLedger::record_child_grant(child, Cycles::new(30), 40);

        let events = CycleTopupEvents::data(0, usize::MAX);
        let funding = CyclesFundingLedger::export();
        CycleTopupEvents::clear_for_tests();
        CyclesFundingLedger::clear_for_tests();

        CycleTopupEvents::import(events.clone());
        CyclesFundingLedger::import(funding.clone());
        assert_eq!(CycleTopupEvents::data(0, usize::MAX), events);
        assert_eq!(CyclesFundingLedger::export(), funding);

        CycleTopupEvents::clear_for_tests();
        CyclesFundingLedger::clear_for_tests();
    }

    #[test]
    fn cycle_tracker_latest_and_retention_are_ordered_and_bounded() {
        let _guard = seams::lock();
        let _ = CycleTracker::purge_before(u64::MAX, usize::MAX);
        for timestamp in 1..=4 {
            CycleTracker::record(timestamp, Cycles::new(u128::from(timestamp)));
        }

        assert_eq!(CycleTracker::latest(), Some((4, Cycles::new(4))));
        assert_eq!(CycleTracker::purge_before(4, 2), 2);
        assert_eq!(
            CycleTracker::entries(0, usize::MAX),
            vec![(3, Cycles::new(3)), (4, Cycles::new(4))]
        );
        assert_eq!(CycleTracker::purge_before(4, 2), 1);
        assert_eq!(CycleTracker::latest(), Some((4, Cycles::new(4))));

        let _ = CycleTracker::purge_before(u64::MAX, usize::MAX);
    }
}
