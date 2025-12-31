use crate::{
    cdk::candid::Principal,
    domain::policy,
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    ids::CanisterRole,
    ops::storage::registry::subnet::SubnetRegistryOps,
};
use std::collections::BTreeMap;

///
/// RootAppDirectoryBuilder
///

pub struct RootAppDirectoryBuilder;

impl RootAppDirectoryBuilder {
    #[must_use]
    pub fn build_from_registry() -> AppDirectoryView {
        let entries = SubnetRegistryOps::export_roles();
        let mut map = BTreeMap::<CanisterRole, Principal>::new();

        for (pid, role) in entries {
            if policy::directory::is_app_directory_role(&role) {
                map.insert(role.clone(), pid);
            }
        }

        AppDirectoryView(map.into_iter().collect())
    }
}
///
/// RootSubnetDirectoryBuilder
///

pub struct RootSubnetDirectoryBuilder;

impl RootSubnetDirectoryBuilder {
    #[must_use]
    pub fn build_from_registry() -> SubnetDirectoryView {
        let entries = SubnetRegistryOps::export_roles();
        let mut map = BTreeMap::<CanisterRole, Principal>::new();

        for (pid, role) in entries {
            if policy::directory::is_subnet_directory_role(&role) {
                map.insert(role.clone(), pid);
            }
        }

        SubnetDirectoryView(map.into_iter().collect())
    }
}
