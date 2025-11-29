use crate::{
    config::Config,
    model::memory::{
        Env,
        directory::{AppDirectory, DirectoryView, PrincipalList},
        topology::SubnetCanisterRegistry,
    },
    ops::{
        model::memory::directory::{DirectoryPageDto, paginate},
        prelude::*,
    },
};
use std::collections::BTreeMap;

///
/// AppDirectoryOps
///

pub struct AppDirectoryOps;

impl AppDirectoryOps {
    #[must_use]
    pub fn export() -> DirectoryView {
        if Env::is_root() {
            Self::root_build_view()
        } else {
            AppDirectory::export()
        }
    }

    #[must_use]
    pub fn page(offset: u64, limit: u64) -> DirectoryPageDto {
        let view = AppDirectory::export();
        let total = view.len() as u64;

        DirectoryPageDto {
            entries: paginate(view, offset, limit),
            total,
        }
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
}
