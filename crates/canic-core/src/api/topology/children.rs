use crate::{
    dto::{
        canister::{CanisterChildView, CanisterSummaryView},
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    workflow::topology::children::query::CanisterChildrenQuery,
};

///
/// CanisterChildrenApi
///

pub struct CanisterChildrenApi;

impl CanisterChildrenApi {
    #[must_use]
    pub fn page(page: PageRequest) -> Page<CanisterSummaryView> {
        CanisterChildrenQuery::page(page)
    }

    #[must_use]
    pub fn find_first_by_role(role: &CanisterRole) -> Option<CanisterChildView> {
        CanisterChildrenQuery::find_first_by_role(role)
    }
}
