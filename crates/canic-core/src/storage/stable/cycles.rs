use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        types::Cycles,
    },
    eager_static,
    storage::stable::memory::cycles::CYCLE_TRACKER_ID,
};
use canic_memory::ic_memory;
use std::cell::RefCell;

eager_static! {
    //
    // CYCLE_TRACKER
    //
    static CYCLE_TRACKER: RefCell<CycleTracker> =
        RefCell::new(CycleTracker::new(BTreeMap::init(
            ic_memory!(CycleTracker, CYCLE_TRACKER_ID),
        )));
}

///
/// CycleTracker
///

pub struct CycleTracker {
    map: BTreeMap<u64, Cycles, VirtualMemory<DefaultMemoryImpl>>,
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
