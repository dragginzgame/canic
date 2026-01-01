use crate::{
    cdk::candid::Principal,
    domain::policy,
    ids::CanisterRole,
    ops::storage::{
        directory::{app::AppDirectorySnapshot, subnet::SubnetDirectorySnapshot},
        registry::subnet::SubnetRegistryOps,
    },
};
use std::collections::BTreeMap;

///
/// RootAppDirectoryBuilder
///

pub struct RootAppDirectoryBuilder;

impl RootAppDirectoryBuilder {
    #[must_use]
    pub fn build_from_registry() -> AppDirectorySnapshot {
        let entries = SubnetRegistryOps::export_roles();
        let mut map = BTreeMap::<CanisterRole, Principal>::new();

        for (pid, role) in entries {
            if policy::directory::is_app_directory_role(&role) {
                map.insert(role.clone(), pid);
            }
        }

        AppDirectorySnapshot {
            entries: map.into_iter().collect(),
        }
    }
}
///
/// RootSubnetDirectoryBuilder
///

pub struct RootSubnetDirectoryBuilder;

impl RootSubnetDirectoryBuilder {
    #[must_use]
    pub fn build_from_registry() -> SubnetDirectorySnapshot {
        let entries = SubnetRegistryOps::export_roles();
        let mut map = BTreeMap::<CanisterRole, Principal>::new();

        for (pid, role) in entries {
            if policy::directory::is_subnet_directory_role(&role) {
                map.insert(role.clone(), pid);
            }
        }

        SubnetDirectorySnapshot {
            entries: map.into_iter().collect(),
        }
    }
}
