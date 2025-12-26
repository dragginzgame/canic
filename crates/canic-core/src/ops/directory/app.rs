use crate::{
    config::Config,
    dto::page::{Page, PageRequest},
    model::memory::{directory::AppDirectory, topology::SubnetCanisterRegistry},
    ops::{
        prelude::*,
        storage::{
            directory::{DirectoryView, paginate},
            env::EnvOps,
        },
    },
};
use candid::Principal;
use std::collections::BTreeMap;

///
/// AppDirectoryOps
///

pub struct AppDirectoryOps;

impl AppDirectoryOps {
    /// Single source of truth: recompute on root, otherwise use stable view.
    fn resolve_view() -> DirectoryView {
        if EnvOps::is_root() {
            Self::root_build_view()
        } else {
            AppDirectory::view()
        }
    }

    /// Fetch principal for a canister role from the current AppDirectory.
    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        let target = role.clone();
        let view = Self::resolve_view();

        view.into_iter()
            .find_map(|(t, pid)| (t == target).then_some(pid))
    }

    /// Public stable API for exporting the view.
    #[must_use]
    pub fn export() -> DirectoryView {
        Self::resolve_view()
    }

    /// Import a full view into stable memory.
    pub fn import(view: DirectoryView) {
        AppDirectory::import(view);
    }

    #[must_use]
    pub fn page(request: PageRequest) -> Page<(CanisterRole, Principal)> {
        paginate(Self::resolve_view(), request)
    }

    /// Build AppDirectory from the registry.
    #[must_use]
    pub fn root_build_view() -> DirectoryView {
        let cfg = Config::get();
        let entries = SubnetCanisterRegistry::export();
        let mut map: BTreeMap<CanisterRole, Principal> = BTreeMap::new();

        for entry in entries {
            let role = entry.role.clone();

            if cfg.app_directory.contains(&role) {
                map.insert(role, entry.pid);
            }
        }

        map.into_iter().collect()
    }
}
