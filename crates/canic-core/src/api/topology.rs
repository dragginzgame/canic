use crate::{
    PublicError,
    cdk::types::Principal,
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
        topology::{AppRegistryView, DirectoryEntryView, SubnetRegistryView},
    },
    ids::CanisterRole,
    workflow,
};

///
/// Registry
///

#[must_use]
pub fn app_registry() -> AppRegistryView {
    workflow::topology::registry::query::app_registry_view()
}

#[must_use]
pub fn subnet_registry() -> SubnetRegistryView {
    workflow::topology::registry::query::subnet_registry_view()
}

///
/// Directory
///

#[must_use]
pub fn app_directory(page: PageRequest) -> Page<DirectoryEntryView> {
    workflow::topology::directory::query::app_directory_page(page)
}

#[must_use]
pub fn subnet_directory(page: PageRequest) -> Page<DirectoryEntryView> {
    workflow::topology::directory::query::subnet_directory_page(page)
}

///
/// Children
///

#[must_use]
pub fn canister_children(page: PageRequest) -> Page<CanisterSummaryView> {
    workflow::topology::children::query::canister_children_page(page)
}

/// Lookup the first direct child matching the role in the children cache.
///
/// Returns `None` when no matching child is cached.
pub fn child_by_role(role: CanisterRole) -> Result<Option<Principal>, PublicError> {
    workflow::topology::children::query::child_pid_by_role(role).map_err(PublicError::from)
}
