use crate::{
    cdk::types::Cycles,
    dto::page::{Page, PageRequest},
    workflow,
};

///
/// Cycles API
///

#[must_use]
pub fn cycle_tracker(page: PageRequest) -> Page<(u64, Cycles)> {
    workflow::runtime::cycles::query::cycle_tracker_page(page)
}
