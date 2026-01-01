use crate::{
    cdk::types::Cycles,
    dto::page::{Page, PageRequest},
    ops::storage::cycles::CycleTrackerOps,
    workflow::view::paginate::paginate_vec,
};

pub(crate) fn cycle_tracker_page(page: PageRequest) -> Page<(u64, Cycles)> {
    let snapshot = CycleTrackerOps::snapshot();
    paginate_vec(snapshot.entries, page)
}
