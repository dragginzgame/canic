use crate::{
    dto::{
        cycles::CycleTrackerEntry,
        page::{Page, PageRequest},
    },
    ops::storage::cycles::CycleTrackerOps,
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
}
