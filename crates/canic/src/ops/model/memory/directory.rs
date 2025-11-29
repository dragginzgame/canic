use crate::{
    Error,
    config::Config,
    model::memory::{
        directory::{DirectoryView, PrincipalList},
        topology::SubnetCanisterRegistry,
    },
    ops::prelude::*,
};
use std::collections::BTreeMap;

/// Build AppDirectory from the registry.
pub(crate) fn build_app_directory_view() -> DirectoryView {
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

/// Build SubnetDirectory from the registry.
pub(crate) fn build_subnet_directory_view() -> Result<DirectoryView, Error> {
    let cfg = cfg_current_subnet()?;
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
