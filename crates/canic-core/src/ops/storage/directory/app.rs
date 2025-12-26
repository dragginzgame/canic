use crate::{
    dto::page::{Page, PageRequest},
    model::memory::directory::{AppDirectory, DirectoryView},
    ops::{prelude::*, storage::directory::paginate},
};
use candid::Principal;

///
/// AppDirectoryOps
///

pub struct AppDirectoryOps;

impl AppDirectoryOps {
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        AppDirectory::view()
            .into_iter()
            .find_map(|(t, pid)| (t == *role).then_some(pid))
    }

    pub fn export() -> DirectoryView {
        AppDirectory::view()
    }

    pub fn import(view: DirectoryView) {
        AppDirectory::import(view);
    }

    pub fn page(request: PageRequest) -> Page<(CanisterRole, Principal)> {
        paginate(AppDirectory::view(), request)
    }
}
