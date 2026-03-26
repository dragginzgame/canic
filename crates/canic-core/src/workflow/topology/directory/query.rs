use crate::{
    dto::{
        page::{Page, PageRequest},
        topology::DirectoryEntryResponse,
    },
    ops::storage::directory::{
        app::AppDirectoryOps, mapper::DirectoryResponseMapper, subnet::SubnetDirectoryOps,
    },
    workflow::{prelude::*, view::paginate::paginate_vec},
};

///
/// AppDirectoryQuery
///

pub struct AppDirectoryQuery;

impl AppDirectoryQuery {
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        AppDirectoryOps::get(&role)
    }

    #[must_use]
    pub fn page(page: PageRequest) -> Page<DirectoryEntryResponse> {
        directory_page(AppDirectoryOps::data().entries, page)
    }
}

///
/// SubnetDirectoryQuery
///

pub struct SubnetDirectoryQuery;

impl SubnetDirectoryQuery {
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        SubnetDirectoryOps::get(&role)
    }

    #[must_use]
    pub fn page(page: PageRequest) -> Page<DirectoryEntryResponse> {
        directory_page(SubnetDirectoryOps::data().entries, page)
    }
}

// Paginate directory tuples and let ops own the tuple -> DTO mapping.
fn directory_page(
    entries: Vec<(CanisterRole, Principal)>,
    page: PageRequest,
) -> Page<DirectoryEntryResponse> {
    DirectoryResponseMapper::record_page_to_response(paginate_vec(entries, page))
}
