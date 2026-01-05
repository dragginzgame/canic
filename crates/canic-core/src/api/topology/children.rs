use crate::{
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
    },
    workflow,
};

///
/// CanisterChildrenApi
///

pub struct CanisterChildrenApi;

impl CanisterChildrenApi {
    #[must_use]
    pub fn page(page: PageRequest) -> Page<CanisterSummaryView> {
        workflow::topology::children::query::CanisterChildrenQuery::page(page)
    }
}
