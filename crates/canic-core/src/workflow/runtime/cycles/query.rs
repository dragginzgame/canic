use crate::{
    dto::{
        cycles::{CycleTopupEvent, CycleTrackerEntry},
        page::{Page, PageRequest},
    },
    ops::storage::cycles::{CycleTopupEventOps, CycleTrackerOps},
    workflow::view::paginate::paginate_vec,
};

///
/// CycleTrackerQuery
///

pub struct CycleTrackerQuery;

impl CycleTrackerQuery {
    #[must_use]
    pub fn page(page: PageRequest) -> Page<CycleTrackerEntry> {
        CycleTrackerOps::page_to_response(paginate_vec(CycleTrackerOps::entries(), page))
    }

    #[must_use]
    pub fn topups(page: PageRequest) -> Page<CycleTopupEvent> {
        CycleTopupEventOps::page_to_response(paginate_vec(CycleTopupEventOps::entries(), page))
    }
}
