use crate::{
    dto::{
        directory::DirectoryView,
        page::{Page, PageRequest},
    },
    ids::CanisterRole,
    model::memory::directory::SubnetDirectory,
    ops::storage::directory::paginate,
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
        paginate(SubnetDirectory::export(), request)
    }

    #[must_use]
    pub fn export() -> DirectoryView {
        SubnetDirectory::export()
    }

    pub fn import(view: DirectoryView) {
        SubnetDirectory::import(view);
    }
}
