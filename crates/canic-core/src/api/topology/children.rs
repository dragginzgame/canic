use crate::{
    dto::{
        canister::CanisterRecordView,
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
    pub fn page(page: PageRequest) -> Page<CanisterRecordView> {
        CanisterChildrenQuery::page(page)
    }

    #[must_use]
    pub fn find_first_by_role(role: &CanisterRole) -> Option<CanisterRecordView> {
        CanisterChildrenQuery::find_first_by_role(role)
    }
}
