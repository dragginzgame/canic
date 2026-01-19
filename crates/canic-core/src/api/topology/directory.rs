use crate::{
    cdk::types::Principal,
    dto::{
        page::{Page, PageRequest},
        topology::DirectoryEntryResponse,
    },
    ids::CanisterRole,
    workflow::topology::directory::query::{AppDirectoryQuery, SubnetDirectoryQuery},
};

///
/// AppDirectoryApi
///

pub struct AppDirectoryApi;

impl AppDirectoryApi {
    #[must_use]
    pub fn page(page: PageRequest) -> Page<DirectoryEntryResponse> {
        AppDirectoryQuery::page(page)
    }

    /// Lookup the app directory entry for the given role.
    ///
    /// Returns `None` when the role is not present in the directory.
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        AppDirectoryQuery::get(role)
    }
}

///
/// SubnetDirectoryApi
///

pub struct SubnetDirectoryApi;

impl SubnetDirectoryApi {
    #[must_use]
    pub fn page(page: PageRequest) -> Page<DirectoryEntryResponse> {
        SubnetDirectoryQuery::page(page)
    }

    /// Lookup the subnet directory entry for the given role.
    /// Returns `None` when the role is not present in the directory.
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        SubnetDirectoryQuery::get(role)
    }
}
