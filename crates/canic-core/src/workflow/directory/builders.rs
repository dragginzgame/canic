use crate::{
    cdk::candid::Principal, ids::CanisterRole, ops::storage::registry::SubnetRegistryOps, policy,
};
use std::collections::BTreeMap;

///
/// RootAppDirectoryBuilder
///

pub struct RootAppDirectoryBuilder;

impl RootAppDirectoryBuilder {
    #[must_use]
    pub fn build_from_registry() -> Vec<(CanisterRole, Principal)> {
        let entries = SubnetRegistryOps::export();
        let mut map = BTreeMap::<CanisterRole, Principal>::new();

        for (pid, entry) in entries {
            if policy::directory::is_app_directory_role(&entry.role) {
                map.insert(entry.role.clone(), pid);
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
    pub fn build_from_registry() -> Vec<(CanisterRole, Principal)> {
        let entries = SubnetRegistryOps::export();
        let mut map = BTreeMap::<CanisterRole, Principal>::new();

        for (pid, entry) in entries {
            if policy::directory::is_subnet_directory_role(&entry.role) {
                map.insert(entry.role.clone(), pid);
            }
        }

        map.into_iter().collect()
    }
}
