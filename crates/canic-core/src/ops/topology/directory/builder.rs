//! Module: ops::topology::directory::builder
//!
//! Responsibility: build root-derived Fleet and Subnet Directories from registry state.
//! Does not own: registry storage, Directory import, or endpoint DTO schemas.
//! Boundary: deterministic ops helper used by root Directory resolvers.

use crate::{
    InternalError,
    cdk::types::Principal,
    ids::CanisterRole,
    ops::storage::directory::DirectoryOpsError,
    storage::canister::CanisterRecord,
    storage::stable::{
        directory::{DirectoryEntryRecord, fleet::FleetDirectoryData, subnet::SubnetDirectoryData},
        registry::subnet::SubnetRegistryData,
    },
};
use std::collections::{BTreeMap, BTreeSet};

///
/// RootFleetDirectoryBuilder
///
/// Operations-layer builder for the root-derived Fleet Directory.
///

pub struct RootFleetDirectoryBuilder;

impl RootFleetDirectoryBuilder {
    pub fn build(
        registry: &SubnetRegistryData,
        fleet_roles: &BTreeSet<CanisterRole>,
    ) -> Result<FleetDirectoryData, InternalError> {
        let mut entries = BTreeMap::new();

        for record in registry
            .entries
            .iter()
            .filter(|record| is_direct_root_child(registry, &record.record))
            .filter(|record| fleet_roles.contains(&record.record.role))
        {
            if entries
                .insert(record.record.role.clone(), record.pid)
                .is_some()
            {
                return Err(DirectoryOpsError::DuplicateRole {
                    directory: "Fleet",
                    role: record.record.role.clone(),
                }
                .into());
            }
        }

        Ok(FleetDirectoryData {
            entries: entries
                .into_iter()
                .map(|(role, pid)| DirectoryEntryRecord { role, pid })
                .collect(),
        })
    }
}

///
/// RootSubnetDirectoryBuilder
///
/// Operations-layer builder for the root-derived Subnet Directory.
///

pub struct RootSubnetDirectoryBuilder;

impl RootSubnetDirectoryBuilder {
    pub fn build(
        registry: &SubnetRegistryData,
        subnet_roles: &BTreeSet<CanisterRole>,
    ) -> Result<SubnetDirectoryData, InternalError> {
        let mut entries = BTreeMap::new();

        for record in registry
            .entries
            .iter()
            .filter(|record| is_direct_root_child(registry, &record.record))
            .filter(|record| subnet_roles.contains(&record.record.role))
        {
            if entries
                .insert(record.record.role.clone(), record.pid)
                .is_some()
            {
                return Err(DirectoryOpsError::DuplicateRole {
                    directory: "Subnet",
                    role: record.record.role.clone(),
                }
                .into());
            }
        }

        Ok(SubnetDirectoryData {
            entries: entries
                .into_iter()
                .map(|(role, pid)| DirectoryEntryRecord { role, pid })
                .collect(),
        })
    }
}

fn root_pid(registry: &SubnetRegistryData) -> Option<Principal> {
    registry
        .entries
        .iter()
        .find(|entry| entry.record.role == CanisterRole::ROOT && entry.record.parent_pid.is_none())
        .map(|entry| entry.pid)
}

fn is_direct_root_child(registry: &SubnetRegistryData, entry: &CanisterRecord) -> bool {
    root_pid(registry).is_some_and(|root| entry.parent_pid == Some(root))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::canister::{CanisterEntryRecord, CanisterRecord};

    fn p(n: u8) -> Principal {
        Principal::from_slice(&[n])
    }

    fn record(role: &str, parent_pid: Option<Principal>) -> CanisterRecord {
        CanisterRecord {
            role: CanisterRole::owned(role.to_string()),
            parent_pid,
            module_hash: None,
            created_at: 0,
        }
    }

    fn registry(entries: Vec<(Principal, CanisterRecord)>) -> SubnetRegistryData {
        SubnetRegistryData {
            entries: entries
                .into_iter()
                .map(|(pid, record)| CanisterEntryRecord { pid, record })
                .collect(),
        }
    }

    #[test]
    fn subnet_directory_ignores_nested_matching_roles_before_duplicate_detection() {
        let root = p(0);
        let direct_service = p(1);
        let nested_parent = p(2);
        let nested_service = p(3);
        let roles = BTreeSet::from([CanisterRole::from("project_hub")]);
        let registry = registry(vec![
            (root, record("root", None)),
            (direct_service, record("project_hub", Some(root))),
            (nested_parent, record("project_instance", Some(root))),
            (nested_service, record("project_hub", Some(nested_parent))),
        ]);

        let directory = RootSubnetDirectoryBuilder::build(&registry, &roles)
            .expect("nested matching role should not duplicate root service");

        assert_eq!(
            directory.entries,
            vec![DirectoryEntryRecord {
                role: CanisterRole::from("project_hub"),
                pid: direct_service,
            }]
        );
    }

    #[test]
    fn subnet_directory_rejects_duplicate_direct_root_services() {
        let root = p(0);
        let roles = BTreeSet::from([CanisterRole::from("project_hub")]);
        let registry = registry(vec![
            (root, record("root", None)),
            (p(1), record("project_hub", Some(root))),
            (p(2), record("project_hub", Some(root))),
        ]);

        RootSubnetDirectoryBuilder::build(&registry, &roles)
            .expect_err("duplicate direct root services should fail");
    }

    #[test]
    fn subnet_directory_excludes_stale_direct_root_roles_not_configured_for_directory() {
        let root = p(0);
        let direct_service = p(1);
        let stale_singleton_residue = p(2);
        let roles = BTreeSet::from([CanisterRole::from("project_hub")]);
        let registry = registry(vec![
            (root, record("root", None)),
            (direct_service, record("project_hub", Some(root))),
            (
                stale_singleton_residue,
                record("project_ledger", Some(root)),
            ),
        ]);

        let directory = RootSubnetDirectoryBuilder::build(&registry, &roles)
            .expect("stale direct root singleton residue should be excluded");

        assert_eq!(
            directory.entries,
            vec![DirectoryEntryRecord {
                role: CanisterRole::from("project_hub"),
                pid: direct_service,
            }]
        );
    }

    #[test]
    fn fleet_directory_ignores_nested_matching_roles_before_duplicate_detection() {
        let root = p(0);
        let direct_service = p(1);
        let nested_parent = p(2);
        let nested_service = p(3);
        let roles = BTreeSet::from([CanisterRole::from("project_hub")]);
        let registry = registry(vec![
            (root, record("root", None)),
            (direct_service, record("project_hub", Some(root))),
            (nested_parent, record("project_instance", Some(root))),
            (nested_service, record("project_hub", Some(nested_parent))),
        ]);

        let directory = RootFleetDirectoryBuilder::build(&registry, &roles)
            .expect("nested matching role should not duplicate Fleet service");

        assert_eq!(
            directory.entries,
            vec![DirectoryEntryRecord {
                role: CanisterRole::from("project_hub"),
                pid: direct_service,
            }]
        );
    }

    #[test]
    fn fleet_directory_excludes_stale_direct_root_roles_not_configured_for_directory() {
        let root = p(0);
        let direct_service = p(1);
        let stale_singleton_residue = p(2);
        let roles = BTreeSet::from([CanisterRole::from("project_hub")]);
        let registry = registry(vec![
            (root, record("root", None)),
            (direct_service, record("project_hub", Some(root))),
            (
                stale_singleton_residue,
                record("project_ledger", Some(root)),
            ),
        ]);

        let directory = RootFleetDirectoryBuilder::build(&registry, &roles)
            .expect("stale direct root singleton residue should be excluded");

        assert_eq!(
            directory.entries,
            vec![DirectoryEntryRecord {
                role: CanisterRole::from("project_hub"),
                pid: direct_service,
            }]
        );
    }
}
