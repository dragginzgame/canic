use crate::{
    cdk::types::Principal,
    dto::{
        canister::CanisterSummaryView,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    ops::storage::{
        children::CanisterChildrenOps,
        directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
    },
};

pub(crate) fn app_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    AppDirectoryOps::page(page)
}

pub(crate) fn subnet_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    SubnetDirectoryOps::page(page)
}

pub(crate) fn subnet_canister_children_page(page: PageRequest) -> Page<CanisterSummaryView> {
    CanisterChildrenOps::page(page)
}
