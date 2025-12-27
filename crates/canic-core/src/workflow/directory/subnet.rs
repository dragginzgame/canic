use crate::{
    ops::{
        env::EnvOps,
        storage::{
            directory::{DirectoryView, SubnetDirectoryOps},
            topology::subnet::SubnetRegistryOps,
        },
    },
    policy,
};
use std::collections::BTreeMap;

///
/// SubnetDirectoryWorkflow
///

pub struct SubnetDirectoryWorkflow;

impl SubnetDirectoryWorkflow {
    pub fn resolve_view() -> DirectoryView {
        if EnvOps::is_root() {
            Self::build_from_registry()
        } else {
            SubnetDirectoryOps::export()
        }
    }

    pub fn build_from_registry() -> DirectoryView {
        let entries = SubnetRegistryOps::export();
        let mut map = BTreeMap::new();

        for entry in entries {
            if policy::directory::is_subnet_directory_role(&entry.role) {
                map.insert(entry.role.clone(), entry.pid);
            }
        }

        map.into_iter().collect()
    }
}
