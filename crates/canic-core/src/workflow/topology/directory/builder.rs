use crate::{
    ids::CanisterRole,
    ops::storage::{
        directory::{app::AppDirectorySnapshot, subnet::SubnetDirectorySnapshot},
        registry::subnet::SubnetRegistrySnapshot,
    },
};
use std::collections::{BTreeMap, BTreeSet};

///
/// RootAppDirectoryBuilder
///

pub struct RootAppDirectoryBuilder;

impl RootAppDirectoryBuilder {
    #[must_use]
    pub fn build(
        registry: &SubnetRegistrySnapshot,
        app_roles: &BTreeSet<CanisterRole>,
    ) -> AppDirectorySnapshot {
        let entries = registry
            .entries
            .iter()
            .filter(|(_, entry)| app_roles.contains(&entry.role))
            .map(|(pid, entry)| (entry.role.clone(), *pid))
            .collect::<BTreeMap<_, _>>();

        AppDirectorySnapshot {
            entries: entries.into_iter().collect(),
        }
    }
}

///
/// RootSubnetDirectoryBuilder
///

pub struct RootSubnetDirectoryBuilder;

impl RootSubnetDirectoryBuilder {
    #[must_use]
    pub fn build(
        registry: &SubnetRegistrySnapshot,
        subnet_roles: &BTreeSet<CanisterRole>,
    ) -> SubnetDirectorySnapshot {
        let entries = registry
            .entries
            .iter()
            .filter(|(_, entry)| subnet_roles.contains(&entry.role))
            .map(|(pid, entry)| (entry.role.clone(), *pid))
            .collect::<BTreeMap<_, _>>();

        SubnetDirectorySnapshot {
            entries: entries.into_iter().collect(),
        }
    }
}
