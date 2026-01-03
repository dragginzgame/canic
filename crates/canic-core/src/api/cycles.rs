use crate::{
    PublicError,
    cdk::types::Cycles,
    dto::page::{Page, PageRequest},
    workflow,
};

pub fn canic_cycle_tracker(page: PageRequest) -> Result<Page<(u64, Cycles)>, PublicError> {
    Ok(workflow::runtime::cycles::query::cycle_tracker_page(page))
}
