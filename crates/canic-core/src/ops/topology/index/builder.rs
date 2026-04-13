use crate::{
    InternalError,
    ids::CanisterRole,
    ops::storage::index::IndexOpsError,
    storage::stable::{
        index::{app::AppIndexRecord, subnet::SubnetIndexRecord},
        registry::subnet::SubnetRegistryRecord,
    },
};
use std::collections::{BTreeMap, BTreeSet};

///
/// RootAppIndexBuilder
///

pub struct RootAppIndexBuilder;

impl RootAppIndexBuilder {
    pub fn build(
        registry: &SubnetRegistryRecord,
        app_roles: &BTreeSet<CanisterRole>,
    ) -> Result<AppIndexRecord, InternalError> {
        let mut entries = BTreeMap::new();

        for (pid, entry) in registry
            .entries
            .iter()
            .filter(|(_, entry)| app_roles.contains(&entry.role))
        {
            if entries.insert(entry.role.clone(), *pid).is_some() {
                return Err(IndexOpsError::DuplicateRole {
                    index: "app",
                    role: entry.role.clone(),
                }
                .into());
            }
        }

        Ok(AppIndexRecord {
            entries: entries.into_iter().collect(),
        })
    }
}

///
/// RootSubnetIndexBuilder
///

pub struct RootSubnetIndexBuilder;

impl RootSubnetIndexBuilder {
    pub fn build(
        registry: &SubnetRegistryRecord,
        subnet_roles: &BTreeSet<CanisterRole>,
    ) -> Result<SubnetIndexRecord, InternalError> {
        let mut entries = BTreeMap::new();

        for (pid, entry) in registry
            .entries
            .iter()
            .filter(|(_, entry)| subnet_roles.contains(&entry.role))
        {
            if entries.insert(entry.role.clone(), *pid).is_some() {
                return Err(IndexOpsError::DuplicateRole {
                    index: "subnet",
                    role: entry.role.clone(),
                }
                .into());
            }
        }

        Ok(SubnetIndexRecord {
            entries: entries.into_iter().collect(),
        })
    }
}
