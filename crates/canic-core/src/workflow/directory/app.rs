use crate::{
    ops::{
        env::EnvOps,
        storage::{
            directory::{AppDirectoryOps, DirectoryView},
            topology::SubnetRegistryOps,
        },
    },
    policy,
};
use std::collections::BTreeMap;

///
/// AppDirectoryWorkflow
///

pub struct AppDirectoryWorkflow;

impl AppDirectoryWorkflow {
    pub fn rebuild_from_registry() -> DirectoryView {
        let entries = SubnetRegistryOps::export();
        let mut map = BTreeMap::new();

        for entry in entries {
            if policy::directory::is_app_directory_role(&entry.role) {
                map.insert(entry.role.clone(), entry.pid);
            }
        }

        map.into_iter().collect()
    }

    pub fn resolve_view() -> DirectoryView {
        if EnvOps::is_root() {
            Self::rebuild_from_registry()
        } else {
            AppDirectoryOps::export()
        }
    }
}
