use crate::{
    dto::{
        page::{Page, PageRequest},
        topology::DirectoryEntryView,
    },
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
