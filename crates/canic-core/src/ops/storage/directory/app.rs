use crate::{
    dto::page::{Page, PageRequest},
    model::memory::directory::{AppDirectory, AppDirectoryData},
    ops::{prelude::*, view::paginate_vec},
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
        paginate_vec(AppDirectory::export(), request)
    }

    #[must_use]
    pub fn export() -> AppDirectoryData {
        AppDirectory::export()
    }

    pub fn import(data: AppDirectoryData) {
        AppDirectory::import(data);
    }
}
