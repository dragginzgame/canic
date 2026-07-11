//! Module: workflow::topology::index::query
//!
//! Responsibility: expose read-only app and subnet index pages and role lookups.
//! Does not own: index storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over index storage ops.

use crate::{
    cdk::types::Principal,
    dto::{
        page::{Page, PageRequest},
        topology::IndexEntryResponse,
    },
    ids::CanisterRole,
    ops::storage::index::{app::AppIndexOps, mapper::IndexEntryMapper, subnet::SubnetIndexOps},
    storage::stable::index::IndexEntryRecord,
    workflow::view::paginate::paginate_vec,
};

///
/// AppIndexQuery
///

pub struct AppIndexQuery;

impl AppIndexQuery {
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        AppIndexOps::get(&role)
    }

    #[must_use]
    pub fn page(page: PageRequest) -> Page<IndexEntryResponse> {
        index_page(AppIndexOps::data().entries, page)
    }
}

///
/// SubnetIndexQuery
///

pub struct SubnetIndexQuery;

impl SubnetIndexQuery {
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        SubnetIndexOps::get(&role)
    }

    #[must_use]
    pub fn page(page: PageRequest) -> Page<IndexEntryResponse> {
        index_page(SubnetIndexOps::data().entries, page)
    }
}

// Paginate index records and let ops own record -> DTO mapping.
fn index_page(entries: Vec<IndexEntryRecord>, page: PageRequest) -> Page<IndexEntryResponse> {
    IndexEntryMapper::record_page_to_response(paginate_vec(entries, page))
}
