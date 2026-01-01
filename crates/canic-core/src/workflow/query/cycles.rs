use crate::{
    cdk::types::Cycles,
    dto::page::{Page, PageRequest},
    ops::storage::cycles::CycleTrackerOps,
};

pub(crate) fn cycle_tracker_page(page: PageRequest) -> Page<(u64, Cycles)> {
    CycleTrackerOps::page(page)
}
