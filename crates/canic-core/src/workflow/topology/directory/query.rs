use crate::{
    dto::{
        page::{Page, PageRequest},
        topology::DirectoryEntryView,
    },
    ops::storage::directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
    workflow::{prelude::*, view::paginate::paginate_vec},
};

///
/// AppDirectoryQuery
///

pub struct AppDirectoryQuery;

impl AppDirectoryQuery {
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        AppDirectoryOps::get(&role)
    }

    pub fn page(page: PageRequest) -> Page<DirectoryEntryView> {
        let data = AppDirectoryOps::data();
        map_directory_page(paginate_vec(data.entries, page))
    }
}

///
/// SubnetDirectoryQuery
///

pub struct SubnetDirectoryQuery;

impl SubnetDirectoryQuery {
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        SubnetDirectoryOps::get(&role)
    }

    pub fn page(page: PageRequest) -> Page<DirectoryEntryView> {
        let data = SubnetDirectoryOps::data();
        map_directory_page(paginate_vec(data.entries, page))
    }
}

fn map_directory_page(page: Page<(CanisterRole, Principal)>) -> Page<DirectoryEntryView> {
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
