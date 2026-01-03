use crate::{
    cdk::types::Principal,
    dto::{
        page::{Page, PageRequest},
        topology::DirectoryEntryView,
    },
    ids::CanisterRole,
    ops::storage::directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
    workflow::view::paginate::paginate_vec,
};

///
/// Pagination
///

pub fn app_directory_page(page: PageRequest) -> Page<DirectoryEntryView> {
    let snapshot = AppDirectoryOps::snapshot();
    map_directory_page(paginate_vec(snapshot.entries, page))
}

pub fn subnet_directory_page(page: PageRequest) -> Page<DirectoryEntryView> {
    let snapshot = SubnetDirectoryOps::snapshot();
    map_directory_page(paginate_vec(snapshot.entries, page))
}

#[must_use]
pub fn subnet_directory_pid_by_role(role: CanisterRole) -> Option<Principal> {
    SubnetDirectoryOps::get(&role)
}

fn map_directory_page(
    page: Page<(crate::ids::CanisterRole, crate::cdk::types::Principal)>,
) -> Page<DirectoryEntryView> {
    let entries = page
        .entries
        .into_iter()
        .map(|(role, pid)| DirectoryEntryView { role, pid })
        .collect();

    Page {
        entries,
        total: page.total,
    }
}
