use crate::{
    PublicError,
    cdk::types::Principal,
    dto::{
        page::{Page, PageRequest},
        topology::DirectoryEntryView,
    },
    ids::CanisterRole,
    workflow,
};

///
/// Directory API
///

#[must_use]
pub fn app_directory(page: PageRequest) -> Page<DirectoryEntryView> {
    workflow::topology::directory::query::app_directory_page(page)
}

#[must_use]
pub fn subnet_directory(page: PageRequest) -> Page<DirectoryEntryView> {
    workflow::topology::directory::query::subnet_directory_page(page)
}

/// Lookup the app directory entry for the given role.
///
/// Returns `None` when the role is not present in the directory.
pub fn app_directory_pid_by_role(role: CanisterRole) -> Result<Option<Principal>, PublicError> {
    Ok(workflow::topology::directory::query::app_directory_pid_by_role(role))
}

/// Lookup the subnet directory entry for the given role.
///
/// Returns `None` when the role is not present in the directory.
pub fn subnet_directory_pid_by_role(role: CanisterRole) -> Result<Option<Principal>, PublicError> {
    Ok(workflow::topology::directory::query::subnet_directory_pid_by_role(role))
}
