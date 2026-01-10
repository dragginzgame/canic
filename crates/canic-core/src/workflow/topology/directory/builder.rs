use crate::{
    Error,
    ids::CanisterRole,
    ops::storage::{
        directory::DirectoryOpsError,
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
    pub fn build(
        registry: &SubnetRegistrySnapshot,
        app_roles: &BTreeSet<CanisterRole>,
    ) -> Result<AppDirectorySnapshot, Error> {
        let mut entries = BTreeMap::new();

        for (pid, entry) in registry
            .entries
            .iter()
            .filter(|(_, entry)| app_roles.contains(&entry.role))
        {
            if entries.insert(entry.role.clone(), *pid).is_some() {
                return Err(DirectoryOpsError::DuplicateRole {
                    directory: "app",
                    role: entry.role.clone(),
                }
                .into());
            }
        }

        Ok(AppDirectorySnapshot {
            entries: entries.into_iter().collect(),
        })
    }
}

///
/// RootSubnetDirectoryBuilder
///

pub struct RootSubnetDirectoryBuilder;

impl RootSubnetDirectoryBuilder {
    pub fn build(
        registry: &SubnetRegistrySnapshot,
        subnet_roles: &BTreeSet<CanisterRole>,
    ) -> Result<SubnetDirectorySnapshot, Error> {
        let mut entries = BTreeMap::new();

        for (pid, entry) in registry
            .entries
            .iter()
            .filter(|(_, entry)| subnet_roles.contains(&entry.role))
        {
            if entries.insert(entry.role.clone(), *pid).is_some() {
                return Err(DirectoryOpsError::DuplicateRole {
                    directory: "subnet",
                    role: entry.role.clone(),
                }
                .into());
            }
        }

        Ok(SubnetDirectorySnapshot {
            entries: entries.into_iter().collect(),
        })
    }
}
