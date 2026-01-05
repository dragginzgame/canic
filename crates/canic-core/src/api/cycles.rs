use crate::{
    dto::{
        cycles::CycleTrackerEntryView,
        page::{Page, PageRequest},
    },
    workflow::runtime::cycles::query::CycleTrackerQuery,
};

///
/// CycleTrackerApi
///

pub struct CycleTrackerApi;

impl CycleTrackerApi {
    #[must_use]
    pub fn page(page: PageRequest) -> Page<CycleTrackerEntryView> {
        CycleTrackerQuery::page(page)
    }
}
