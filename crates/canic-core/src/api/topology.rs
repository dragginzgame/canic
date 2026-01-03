use crate::{
    PublicError,
    cdk::types::Principal,
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
        topology::{AppRegistryView, SubnetRegistryView},
    },
    ids::CanisterRole,
    workflow,
};

///
/// Registry
///

pub fn canic_app_registry() -> Result<AppRegistryView, PublicError> {
    Ok(workflow::topology::registry::query::app_registry_view())
}

pub fn canic_subnet_registry() -> Result<SubnetRegistryView, PublicError> {
    Ok(workflow::topology::registry::query::subnet_registry_view())
}

///
/// Directory
///

pub fn canic_app_directory(
    page: PageRequest,
) -> Result<Page<(CanisterRole, Principal)>, PublicError> {
    Ok(workflow::topology::directory::query::app_directory_page(
        page,
    ))
}

pub fn canic_subnet_directory(
    page: PageRequest,
) -> Result<Page<(CanisterRole, Principal)>, PublicError> {
    Ok(workflow::topology::directory::query::subnet_directory_page(
        page,
    ))
}

///
/// Children
///

pub fn canic_canister_children(
    page: PageRequest,
) -> Result<Page<CanisterSummaryView>, PublicError> {
    Ok(workflow::topology::children::query::canister_children_page(
        page,
    ))
}

/// Lookup the first direct child matching the role in the children cache.
///
/// Returns `None` when no matching child is cached.
pub fn canic_child_by_role(role: CanisterRole) -> Result<Option<Principal>, PublicError> {
    workflow::topology::children::query::child_pid_by_role(role).map_err(PublicError::from)
}
