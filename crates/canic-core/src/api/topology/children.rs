use crate::{
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
    },
    workflow,
};

///
/// Children API
///

#[must_use]
pub fn canister_children(page: PageRequest) -> Page<CanisterSummaryView> {
    workflow::topology::children::query::canister_children_page(page)
}
