use crate::{
    cdk::structures::{BTreeMap, DefaultMemoryImpl, memory::VirtualMemory},
    core::types::Cycles,
    eager_static, ic_memory, log,
    log::Topic,
    model::memory::id::cycles::CYCLE_TRACKER_ID,
};
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
const RETAIN_SECS: u64 = 60 * 60 * 24 * 7; // ~7 days
const PURGE_INTERVAL: u64 = 1_000; // purge every 1000 inserts

///
/// CycleTrackerView
/// Snapshot view of cycle tracker entries
///

pub type CycleTrackerView = Vec<(u64, Cycles)>;

///
/// CycleTracker
///
/// NOTE : Can't really do tests for this here, it really needs e2e because I can't
/// declare M: Memory as a generic right now, it breaks ic-stable-structures/other ic packages
///

pub(crate) struct CycleTracker {
    map: BTreeMap<u64, u128, VirtualMemory<DefaultMemoryImpl>>,
    insert_count: u64,
}

impl CycleTracker {
    pub const fn new(map: BTreeMap<u64, u128, VirtualMemory<DefaultMemoryImpl>>) -> Self {
        Self {
            map,
            insert_count: 0,
        }
    }

    // -------- PUBLIC API (model-facing) -------- //

    #[must_use]
    pub(crate) fn len() -> u64 {
        CYCLE_TRACKER.with_borrow(|t| t.map.len())
    }

    #[must_use]
    pub(crate) fn record(now: u64, cycles: u128) -> bool {
        CYCLE_TRACKER.with_borrow_mut(|t| t.insert(now, cycles))
    }

    #[must_use]
    pub(crate) fn entries(offset: u64, limit: u64) -> CycleTrackerView {
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);

        CYCLE_TRACKER.with_borrow(|t| {
            t.map
                .iter()
                .skip(offset)
                .take(limit)
                .map(|entry| (*entry.key(), entry.value().into()))
                .collect()
        })
    }

    // -------- INTERNAL MAP OPERATIONS -------- //

    fn insert(&mut self, now: u64, cycles: u128) -> bool {
        self.map.insert(now, cycles);
        self.insert_count += 1;

        if self.insert_count.is_multiple_of(PURGE_INTERVAL) {
            self.purge(now);
        }

        true
    }

    fn purge(&mut self, now: u64) -> usize {
        let cutoff = now.saturating_sub(RETAIN_SECS);
        let mut purged = 0;

        while let Some((first_ts, _)) = self.map.first_key_value() {
            if first_ts < cutoff {
                self.map.remove(&first_ts);
                purged += 1;
            } else {
                break;
            }
        }

        if purged > 0 {
            log!(
                Topic::Cycles,
                Info,
                "cycle_tracker: purged {purged} old entries"
            );
        }

        purged
    }
}
