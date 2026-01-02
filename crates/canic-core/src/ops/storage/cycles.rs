use crate::{cdk::types::Cycles, storage::memory::cycles::CycleTracker};

///
/// CycleTrackerSnapshot
///

#[derive(Clone, Debug)]
pub struct CycleTrackerSnapshot {
    pub entries: Vec<(u64, Cycles)>,
    pub total: u64,
}

///
/// CycleTrackerOps
/// Stable storage wrapper for the cycle tracker.
///

pub struct CycleTrackerOps;

impl CycleTrackerOps {
    #[must_use]
    pub fn len() -> u64 {
        CycleTracker::len()
    }

    pub fn record(now: u64, cycles: Cycles) {
        CycleTracker::record(now, cycles);
    }

    #[must_use]
    pub fn purge(now: u64) -> usize {
        CycleTracker::purge(now)
    }

    #[must_use]
    pub fn snapshot() -> CycleTrackerSnapshot {
        let total = Self::len();
        let entries = CycleTracker::entries(0, usize::MAX);

        CycleTrackerSnapshot { entries, total }
    }
}
