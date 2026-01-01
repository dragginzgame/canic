use crate::{
    cdk::types::Principal,
    dto::{
        directory::{AppDirectoryView, SubnetDirectoryView},
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    ops::storage::directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
    workflow::{
        directory::mapper::{AppDirectoryMapper, SubnetDirectoryMapper},
        view::paginate::paginate_vec,
    },
};

///
/// Views
///

pub fn app_directory_view() -> AppDirectoryView {
    let snapshot = AppDirectoryOps::snapshot();
    AppDirectoryMapper::snapshot_to_view(snapshot)
}

pub fn subnet_directory_view() -> SubnetDirectoryView {
    let snapshot = SubnetDirectoryOps::snapshot();
    SubnetDirectoryMapper::snapshot_to_view(snapshot)
}

///
/// Pagination
///

pub(crate) fn app_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    let snapshot = AppDirectoryOps::snapshot();
    paginate_vec(snapshot.entries, page)
}

pub(crate) fn subnet_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    let snapshot = SubnetDirectoryOps::snapshot();
    paginate_vec(snapshot.entries, page)
}
