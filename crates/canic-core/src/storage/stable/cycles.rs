use crate::{
    cdk::structures::{
        BTreeMap, DefaultMemoryImpl, Storable, memory::VirtualMemory, storable::Bound,
    },
    dto::cycles::CycleTopupEventStatus,
    eager_static,
    storage::{
        prelude::*,
        stable::memory::observability::{CYCLE_TOPUP_EVENTS_ID, CYCLE_TRACKER_ID},
    },
};
use canic_memory::ic_memory;
use std::{borrow::Cow, cell::RefCell};

eager_static! {
    //
    // CYCLE_TRACKER
    //
    static CYCLE_TRACKER: RefCell<CycleTracker> =
        RefCell::new(CycleTracker::new(BTreeMap::init(
            ic_memory!(CycleTracker, CYCLE_TRACKER_ID),
        )));
}

eager_static! {
    //
    // CYCLE_TOPUP_EVENTS
    //
    static CYCLE_TOPUP_EVENTS: RefCell<CycleTopupEvents> =
        RefCell::new(CycleTopupEvents::new(BTreeMap::init(
            ic_memory!(CycleTopupEvents, CYCLE_TOPUP_EVENTS_ID),
        )));
}

///
/// CycleTracker
///

pub struct CycleTracker {
    map: BTreeMap<u64, Cycles, VirtualMemory<DefaultMemoryImpl>>,
}

///
/// CycleTopupEventKey
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
/// CycleTopupEventRecord
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CycleTopupEventRecord {
    pub requested_cycles: Cycles,
    pub transferred_cycles: Option<Cycles>,
    pub status: CycleTopupEventStatus,
    pub error: Option<String>,
}

impl CycleTopupEventRecord {
    pub const STORABLE_MAX_SIZE: u32 = 512;
}

impl_storable_bounded!(
    CycleTopupEventRecord,
    CycleTopupEventRecord::STORABLE_MAX_SIZE,
    false
);

///
/// CycleTopupEvents
///

pub struct CycleTopupEvents {
    map: BTreeMap<CycleTopupEventKey, CycleTopupEventRecord, VirtualMemory<DefaultMemoryImpl>>,
}

impl CycleTopupEvents {
    pub const fn new(
        map: BTreeMap<CycleTopupEventKey, CycleTopupEventRecord, VirtualMemory<DefaultMemoryImpl>>,
    ) -> Self {
        Self { map }
    }

    pub(crate) fn record(
        timestamp_secs: u64,
        requested_cycles: Cycles,
        transferred_cycles: Option<Cycles>,
        status: CycleTopupEventStatus,
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
    pub(crate) fn purge_before(cutoff: u64) -> usize {
        CYCLE_TOPUP_EVENTS.with_borrow_mut(|events| events.purge_inner(cutoff))
    }

    #[must_use]
    pub(crate) fn entries(
        offset: usize,
        limit: usize,
    ) -> Vec<(CycleTopupEventKey, CycleTopupEventRecord)> {
        CYCLE_TOPUP_EVENTS.with_borrow(|events| {
            events
                .map
                .iter()
                .skip(offset)
                .take(limit)
                .map(|entry| (*entry.key(), entry.value()))
                .collect()
        })
    }

    fn purge_inner(&mut self, cutoff: u64) -> usize {
        let mut purged = 0;

        while let Some((first_key, _)) = self.map.first_key_value() {
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
    pub const fn new(map: BTreeMap<u64, Cycles, VirtualMemory<DefaultMemoryImpl>>) -> Self {
        Self { map }
    }

    // -------- PUBLIC API (model-facing) -------- //

    pub(crate) fn record(now: u64, cycles: Cycles) {
        CYCLE_TRACKER.with_borrow_mut(|t| t.insert(now, cycles));
    }

    /// Purge entries older than the provided cutoff timestamp.
    #[must_use]
    pub(crate) fn purge_before(cutoff: u64) -> usize {
        CYCLE_TRACKER.with_borrow_mut(|t| t.purge_inner(cutoff))
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

    // -------- INTERNAL MAP OPERATIONS -------- //

    /// Remove entries older than the provided cutoff timestamp.
    fn purge_inner(&mut self, cutoff: u64) -> usize {
        let mut purged = 0;

        while let Some((first_ts, _)) = self.map.first_key_value() {
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
