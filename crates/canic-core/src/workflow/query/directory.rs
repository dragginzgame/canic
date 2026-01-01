use crate::{
    cdk::types::Principal,
    dto::{
        directory::{AppDirectoryView, SubnetDirectoryView},
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    ops::storage::directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
    workflow::directory::adapter::{app_directory_to_view, subnet_directory_to_view},
};

///
/// Views
///

pub fn app_directory_view() -> AppDirectoryView {
    let entries = AppDirectoryOps::export();
    app_directory_to_view(entries)
}

pub fn subnet_directory_view() -> SubnetDirectoryView {
    let entries = SubnetDirectoryOps::export();
    subnet_directory_to_view(entries)
}

///
/// Pagination
///

pub(crate) fn app_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    AppDirectoryOps::page(page)
}

pub(crate) fn subnet_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    SubnetDirectoryOps::page(page)
}
