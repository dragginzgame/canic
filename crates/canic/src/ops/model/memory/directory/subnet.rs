use crate::{
    Error,
    model::memory::{
        Env,
        directory::{PrincipalList, SubnetDirectory},
        topology::SubnetCanisterRegistry,
    },
    ops::{
        config::ConfigOps,
        model::memory::directory::{DirectoryPageDto, DirectoryView, paginate},
        prelude::*,
    },
};
use std::collections::BTreeMap;

///
/// SubnetDirectoryOps
///

pub struct SubnetDirectoryOps;

impl SubnetDirectoryOps {
    pub fn export() -> Result<DirectoryView, Error> {
        if Env::is_root() {
            Self::root_build_view()
        } else {
            Ok(SubnetDirectory::export())
        }
    }

    pub fn page(offset: u64, limit: u64) -> Result<DirectoryPageDto, Error> {
        let view = Self::export()?;
        let total = view.len() as u64;

        Ok(DirectoryPageDto {
            entries: paginate(view, offset, limit),
            total,
        })
    }

    /// Build SubnetDirectory from the registry.
    pub fn root_build_view() -> Result<DirectoryView, Error> {
        let cfg = ConfigOps::current_subnet()?;
        let entries = SubnetCanisterRegistry::export();

        let mut map: BTreeMap<CanisterType, PrincipalList> = BTreeMap::new();

        for entry in entries {
            let ty = entry.ty.clone();

            if cfg.subnet_directory.contains(&ty) {
                map.entry(ty).or_default().0.push(entry.pid);
            }
        }

        Ok(map.into_iter().collect())
    }
}
