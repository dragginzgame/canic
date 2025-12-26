pub use crate::model::memory::cycles::CycleTrackerView;

use crate::{dto::page::PageRequest, model::memory::cycles::CycleTracker};

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

    pub fn record(now: u64, cycles: u128) {
        CycleTracker::record(now, cycles);
    }

    #[must_use]
    pub fn purge(now: u64) -> usize {
        CycleTracker::purge(now)
    }

    #[must_use]
    pub fn entries(request: PageRequest) -> CycleTrackerView {
        CycleTracker::entries(request)
    }
}
