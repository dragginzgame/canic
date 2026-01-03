use crate::{cdk::types::Cycles, storage::stable::cycles::CycleTracker};

const CYCLE_TRACKER_RETENTION_SECS: u64 = 60 * 60 * 24 * 7; // ~7 days

///
/// CycleTrackerSnapshot
///

#[derive(Clone, Debug)]
pub struct CycleTrackerSnapshot {
    pub entries: Vec<(u64, Cycles)>,
}

///
/// CycleTrackerOps
/// Stable storage wrapper for the cycle tracker.
///

pub struct CycleTrackerOps;

impl CycleTrackerOps {
    pub fn record(now: u64, cycles: Cycles) {
        CycleTracker::record(now, cycles);
    }

    #[must_use]
    pub fn purge(now: u64) -> usize {
        let cutoff = now.saturating_sub(CYCLE_TRACKER_RETENTION_SECS);
        CycleTracker::purge_before(cutoff)
    }

    #[must_use]
    pub fn snapshot() -> CycleTrackerSnapshot {
        let entries = CycleTracker::entries(0, usize::MAX);

        CycleTrackerSnapshot { entries }
    }
}
