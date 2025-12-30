use crate::{
    cdk::types::Cycles,
    dto::page::{Page, PageRequest},
    model::memory::cycles::CycleTracker,
    ops::view::clamp_page_request,
};

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
    pub fn list_entries(request: PageRequest) -> Vec<(u64, Cycles)> {
        let request = clamp_page_request(request);

        let offset = usize::try_from(request.offset).unwrap_or(usize::MAX);
        let limit = usize::try_from(request.limit).unwrap_or(usize::MAX);

        CycleTracker::entries(offset, limit)
    }

    #[must_use]
    pub fn page(request: PageRequest) -> Page<(u64, Cycles)> {
        let entries = Self::list_entries(request);
        let total = Self::len();

        Page { entries, total }
    }
}
