use crate::storage::stable::cycles::CycleTracker;
use crate::{
    dto::{cycles::CycleTrackerEntry, page::Page},
    ops::prelude::*,
};

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
    pub fn purge_before(cutoff: u64) -> usize {
        CycleTracker::purge_before(cutoff)
    }

    #[must_use]
    pub fn entries() -> Vec<(u64, Cycles)> {
        CycleTracker::entries(0, usize::MAX)
    }

    #[must_use]
    pub fn page_to_response(page: Page<(u64, Cycles)>) -> Page<CycleTrackerEntry> {
        Page {
            entries: page
                .entries
                .into_iter()
                .map(|(timestamp_secs, cycles)| CycleTrackerEntry {
                    timestamp_secs,
                    cycles,
                })
                .collect(),
            total: page.total,
        }
    }
}
