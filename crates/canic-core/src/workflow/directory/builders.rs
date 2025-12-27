use crate::{
    model::memory::directory::DirectoryView, ops::storage::registry::SubnetRegistryOps, policy,
};
use std::collections::BTreeMap;

///
/// RootAppDirectoryBuilder
///

pub struct RootAppDirectoryBuilder;

impl RootAppDirectoryBuilder {
    #[must_use]
    pub fn build_from_registry() -> DirectoryView {
        let entries = SubnetRegistryOps::export();
        let mut map = BTreeMap::new();

        for entry in entries {
            if policy::directory::is_app_directory_role(&entry.role) {
                map.insert(entry.role.clone(), entry.pid);
            }
        }

        map.into_iter().collect()
    }
}

///
/// RootSubnetDirectoryBuilder
///

pub struct RootSubnetDirectoryBuilder;

impl RootSubnetDirectoryBuilder {
    #[must_use]
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
