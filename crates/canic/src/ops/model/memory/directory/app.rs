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
        },
        prelude::*,
    },
};
use std::collections::BTreeMap;

///
/// AppDirectoryOpsError
///

#[derive(Debug, ThisError)]
pub enum AppDirectoryOpsError {
    #[error("canister type {0} not found in app directory")]
    NotFound(CanisterType),
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
    /// Public stable API for exporting the view.
    #[must_use]
    pub fn export() -> DirectoryView {
        AppDirectory::export()
    }

    /// Import a full view into stable memory.
    pub fn import(view: DirectoryView) {
        AppDirectory::import(view);
    }

    #[must_use]
    pub fn page(offset: u64, limit: u64) -> DirectoryPageDto {
        paginate(Self::export(), offset, limit)
    }

    /// Build AppDirectory from the registry.
    #[must_use]
    pub fn root_build_view() -> DirectoryView {
        let cfg = Config::get();
        let entries = SubnetCanisterRegistry::export();
        let mut map: BTreeMap<CanisterType, PrincipalList> = BTreeMap::new();

        for entry in entries {
            let ty = entry.ty.clone();

            if cfg.app_directory.contains(&ty) {
                map.entry(ty).or_default().0.push(entry.pid);
            }
        }

        map.into_iter().collect()
    }

    /// Fetch principals for a canister type from the current AppDirectory.
    pub fn try_get(ty: &CanisterType) -> Result<PrincipalList, Error> {
        let entry =
            AppDirectory::get(ty).ok_or_else(|| AppDirectoryOpsError::NotFound(ty.clone()))?;

        Ok(entry)
    }
}
