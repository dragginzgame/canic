use crate::{
    dto::cycles::CycleTrackerEntryView,
    dto::page::{Page, PageRequest},
    workflow,
};

///
/// Cycles API
///

#[must_use]
pub fn cycle_tracker(page: PageRequest) -> Page<CycleTrackerEntryView> {
    workflow::runtime::cycles::query::cycle_tracker_page(page)
}
