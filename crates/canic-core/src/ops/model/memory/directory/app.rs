use crate::{
    Error, ThisError,
    config::Config,
    model::memory::{
        directory::{AppDirectory, PrincipalList},
        topology::SubnetCanisterRegistry,
    },
    ops::{
        model::memory::{
            MemoryOpsError,
            directory::{DirectoryPageDto, DirectoryView, paginate},
            env::EnvOps,
        },
        prelude::*,
    },
    types::PageRequest,
};
use std::collections::BTreeMap;

///
/// AppDirectoryOpsError
///

#[derive(Debug, ThisError)]
pub enum AppDirectoryOpsError {
    #[error("canister type {0} not found in app directory")]
    NotFound(CanisterRole),
}

impl From<AppDirectoryOpsError> for Error {
    fn from(err: AppDirectoryOpsError) -> Self {
        MemoryOpsError::from(err).into()
    }
}

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
    pub fn page(request: PageRequest) -> DirectoryPageDto {
        paginate(Self::resolve_view(), request)
    }

    /// Build AppDirectory from the registry.
    #[must_use]
    pub fn root_build_view() -> DirectoryView {
        let cfg = Config::get();
        let entries = SubnetCanisterRegistry::export();
        let mut map: BTreeMap<CanisterRole, PrincipalList> = BTreeMap::new();

        for entry in entries {
            let role = entry.role.clone();

            if cfg.app_directory.contains(&role) {
                map.entry(role).or_default().0.push(entry.pid);
            }
        }

        map.into_iter().collect()
    }

    /// Fetch principals for a canister type from the current AppDirectory.
    pub fn try_get(role: &CanisterRole) -> Result<PrincipalList, Error> {
        let target = role.clone();
        let view = Self::resolve_view();
        let entry = view
            .into_iter()
            .find_map(|(t, pids)| (t == target).then_some(pids))
            .ok_or_else(|| AppDirectoryOpsError::NotFound(role.clone()))?;

        Ok(entry)
    }
}
