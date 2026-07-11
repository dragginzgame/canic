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
    view::topology::IndexEntryView,
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
        index_page(AppIndexOps::entry_projections(), page)
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
        index_page(SubnetIndexOps::entry_projections(), page)
    }
}

// Paginate read-only index projections and let ops own response mapping.
fn index_page(entries: Vec<IndexEntryView>, page: PageRequest) -> Page<IndexEntryResponse> {
    IndexEntryMapper::projection_page_to_response(paginate_vec(entries, page))
}
