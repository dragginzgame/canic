use crate::{
    cdk::types::Principal,
    dto::page::{Page, PageRequest},
    ids::CanisterRole,
    ops::storage::directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
    workflow::view::paginate::paginate_vec,
};

///
/// Pagination
///

pub fn app_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    let snapshot = AppDirectoryOps::snapshot();
    paginate_vec(snapshot.entries, page)
}

pub fn subnet_directory_page(page: PageRequest) -> Page<(CanisterRole, Principal)> {
    let snapshot = SubnetDirectoryOps::snapshot();
    paginate_vec(snapshot.entries, page)
}
