use crate::{
    cdk::{
        structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
        types::Cycles,
    },
    eager_static,
    model::memory::id::cycles::CYCLE_TRACKER_ID,
};
use canic_memory::ic_memory;
use std::cell::RefCell;

//
// CYCLE_TRACKER
//

eager_static! {
    static CYCLE_TRACKER: RefCell<CycleTracker> =
        RefCell::new(CycleTracker::new(BTreeMap::init(
            ic_memory!(CycleTracker, CYCLE_TRACKER_ID),
        )));
}

/// constants
const STORAGE_RETAIN_SECS: u64 = 60 * 60 * 24 * 7; // ~7 days

///
/// CycleTracker
///
/// NOTE : Can't really do tests for this here, it really needs e2e because I can't
/// declare M: Memory as a generic right now, it breaks ic-stable-structures/other ic packages
///

pub struct CycleTracker {
    map: BTreeMap<u64, Cycles, VirtualMemory<DefaultMemoryImpl>>,
}

impl CycleTracker {
    pub const fn new(map: BTreeMap<u64, Cycles, VirtualMemory<DefaultMemoryImpl>>) -> Self {
        Self { map }
    }

    // -------- PUBLIC API (model-facing) -------- //

    #[must_use]
    pub(crate) fn len() -> u64 {
        CYCLE_TRACKER.with_borrow(|t| t.map.len())
    }

    pub(crate) fn record(now: u64, cycles: Cycles) {
        CYCLE_TRACKER.with_borrow_mut(|t| t.insert(now, cycles));
    }

    /// Purge entries older than the retention window using the shared tracker.
    #[must_use]
    pub(crate) fn purge(now: u64) -> usize {
        CYCLE_TRACKER.with_borrow_mut(|t| t.purge_inner(now))
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

    /// Remove entries older than the retention window.
    fn purge_inner(&mut self, now: u64) -> usize {
        let cutoff = now.saturating_sub(STORAGE_RETAIN_SECS);
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
        self.map.insert(now, cycles);

        true
    }
}
