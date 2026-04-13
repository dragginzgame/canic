use crate::{
    dto::{
        page::{Page, PageRequest},
        topology::IndexEntryResponse,
    },
    ops::storage::index::{app::AppIndexOps, mapper::IndexResponseMapper, subnet::SubnetIndexOps},
    workflow::{prelude::*, view::paginate::paginate_vec},
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

// Paginate index tuples and let ops own the tuple -> DTO mapping.
fn index_page(
    entries: Vec<(CanisterRole, Principal)>,
    page: PageRequest,
) -> Page<IndexEntryResponse> {
    IndexResponseMapper::record_page_to_response(paginate_vec(entries, page))
}
