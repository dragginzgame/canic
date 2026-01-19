use crate::{
    InternalError,
    ids::CanisterRole,
    ops::storage::directory::DirectoryOpsError,
    storage::stable::{
        directory::{app::AppDirectoryRecord, subnet::SubnetDirectoryRecord},
        registry::subnet::SubnetRegistryRecord,
    },
};
use std::collections::{BTreeMap, BTreeSet};

///
/// RootAppDirectoryBuilder
///

pub struct RootAppDirectoryBuilder;

impl RootAppDirectoryBuilder {
    pub fn build(
        registry: &SubnetRegistryRecord,
        app_roles: &BTreeSet<CanisterRole>,
    ) -> Result<AppDirectoryRecord, InternalError> {
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

        Ok(AppDirectoryRecord {
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
        registry: &SubnetRegistryRecord,
        subnet_roles: &BTreeSet<CanisterRole>,
    ) -> Result<SubnetDirectoryRecord, InternalError> {
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

        Ok(SubnetDirectoryRecord {
            entries: entries.into_iter().collect(),
        })
    }
}
