use crate::{
    ops::storage::{
        directory::{DirectoryView, SubnetDirectoryStorageOps},
        env::EnvOps,
        topology::subnet::SubnetCanisterRegistryOps,
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
            SubnetDirectoryStorageOps::export()
        }
    }

    pub fn build_from_registry() -> DirectoryView {
        let entries = SubnetCanisterRegistryOps::export();
        let mut map = BTreeMap::new();

        for entry in entries {
            if policy::directory::is_subnet_directory_role(&entry.role) {
                map.insert(entry.role.clone(), entry.pid);
            }
        }

        map.into_iter().collect()
    }
}
