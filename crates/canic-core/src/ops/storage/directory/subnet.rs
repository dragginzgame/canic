use crate::{
    dto::page::{Page, PageRequest},
    ids::CanisterRole,
    model::memory::directory::{DirectoryView, SubnetDirectory},
    ops::storage::directory::paginate,
};
use candid::Principal;

///
/// SubnetDirectoryOps
///

pub struct SubnetDirectoryOps;

impl SubnetDirectoryOps {
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        SubnetDirectory::view()
            .iter()
            .find_map(|(t, pid)| (t == role).then_some(*pid))
    }

    pub fn export() -> DirectoryView {
        SubnetDirectory::view()
    }

    pub fn import(view: DirectoryView) {
        SubnetDirectory::import(view);
    }

    pub fn page(request: PageRequest) -> Page<(CanisterRole, Principal)> {
        paginate(SubnetDirectory::view(), request)
    }
}
