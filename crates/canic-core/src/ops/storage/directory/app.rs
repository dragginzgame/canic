use crate::{
    dto::{
        directory::AppDirectoryView,
        page::{Page, PageRequest},
    },
    model::memory::directory::{AppDirectory, AppDirectoryData},
    ops::{
        adapter::directory::{app_directory_from_view, app_directory_to_view},
        prelude::*,
        view::paginate_vec,
    },
};

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
    pub(crate) fn export() -> AppDirectoryData {
        AppDirectory::export()
    }

    /// Export app directory as a public view.
    #[must_use]
    pub fn export_view() -> AppDirectoryView {
        let data = AppDirectory::export();
        app_directory_to_view(data)
    }

    pub(crate) fn import(data: AppDirectoryData) {
        AppDirectory::import(data);
    }

    /// Import app directory from a public view.
    pub fn import_view(view: AppDirectoryView) {
        let data = app_directory_from_view(view);
        AppDirectory::import(data);
    }
}
