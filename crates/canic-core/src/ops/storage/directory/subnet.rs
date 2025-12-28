use crate::{
    dto::{
        directory::SubnetDirectoryView,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    model::memory::directory::{SubnetDirectory, SubnetDirectoryData},
    ops::{
        adapter::directory::{subnet_directory_from_view, subnet_directory_to_view},
        view::paginate_vec,
    },
};
use candid::Principal;

///
/// SubnetDirectoryOps
///

pub struct SubnetDirectoryOps;

impl SubnetDirectoryOps {
    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        SubnetDirectory::export()
            .iter()
            .find_map(|(t, pid)| (t == role).then_some(*pid))
    }

    #[must_use]
    pub fn page(request: PageRequest) -> Page<(CanisterRole, Principal)> {
        paginate_vec(SubnetDirectory::export(), request)
    }

    #[must_use]
    pub(crate) fn export() -> SubnetDirectoryData {
        SubnetDirectory::export()
    }

    /// Export subnet directory as a public view.
    #[must_use]
    pub fn export_view() -> SubnetDirectoryView {
        let data = SubnetDirectory::export();
        subnet_directory_to_view(data)
    }

    pub(crate) fn import(data: SubnetDirectoryData) {
        SubnetDirectory::import(data);
    }

    /// Import subnet directory from a public view.
    pub fn import_view(view: SubnetDirectoryView) {
        let data = subnet_directory_from_view(view);
        SubnetDirectory::import(data);
    }
}
