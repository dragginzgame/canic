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
    pub fn get_node_child(role: &CanisterRole) -> Option<CanisterRecordView> {
        CanisterChildrenQuery::get_node_child(role)
    }

    #[must_use]
    pub fn list_children_by_role(role: &CanisterRole) -> Vec<CanisterRecordView> {
        CanisterChildrenQuery::list_children_by_role(role)
    }
}
