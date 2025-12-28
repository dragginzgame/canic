use crate::{
    dto::{
        directory::DirectoryView,
        page::{Page, PageRequest},
    },
    model::memory::directory::AppDirectory,
    ops::{prelude::*, storage::directory::paginate},
};
use candid::Principal;

///
/// AppDirectoryOps
///

pub struct AppDirectoryOps;

impl AppDirectoryOps {
    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        AppDirectory::export()
            .into_iter()
            .find_map(|(t, pid)| (t == *role).then_some(pid))
    }

    #[must_use]
    pub fn page(request: PageRequest) -> Page<(CanisterRole, Principal)> {
        paginate(AppDirectory::export(), request)
    }

    #[must_use]
    pub fn export() -> DirectoryView {
        AppDirectory::export()
    }

    pub fn import(view: DirectoryView) {
        AppDirectory::import(view);
    }
}
