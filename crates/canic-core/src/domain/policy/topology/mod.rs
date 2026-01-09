pub mod registry;

use crate::{
    Error, ThisError, cdk::types::Principal, domain::policy::PolicyError, ids::CanisterRole,
    ops::storage::registry::subnet::SubnetRegistrySnapshot, storage::canister::CanisterRecord,
};
use std::collections::BTreeSet;

///
/// TopologyPolicyError
///

#[derive(Debug, ThisError)]
pub enum TopologyPolicyError {
    #[error("directory entry role mismatch for pid {pid}: expected {expected}, got {found}")]
    DirectoryRoleMismatch {
        pid: Principal,
        expected: CanisterRole,
        found: CanisterRole,
    },

    #[error("directory role {0} appears more than once")]
    DuplicateDirectoryRole(CanisterRole),

    #[error("immediate-parent mismatch: canister {pid} expects parent {expected}, got {found:?}")]
    ImmediateParentMismatch {
        pid: Principal,
        expected: Principal,
        found: Option<Principal>,
    },

    #[error("module hash mismatch for {0}")]
    ModuleHashMismatch(Principal),

    #[error("parent {0} not found in registry")]
    ParentNotFound(Principal),

    #[error("registry entry missing for {0}")]
    RegistryEntryMissing(Principal),

    #[error(transparent)]
    RegistryPolicy(#[from] registry::RegistryPolicyError),
}

impl From<TopologyPolicyError> for Error {
    fn from(err: TopologyPolicyError) -> Self {
        PolicyError::from(err).into()
    }
}

///
/// TopologyPolicy
///

pub struct TopologyPolicy;

impl TopologyPolicy {
    // -------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------

    fn registry_record(
        registry: &'_ SubnetRegistrySnapshot,
        pid: Principal,
    ) -> Result<&'_ CanisterRecord, TopologyPolicyError> {
        registry
            .entries
            .iter()
            .find(|(entry_pid, _)| *entry_pid == pid)
            .map(|(_, record)| record)
            .ok_or(TopologyPolicyError::RegistryEntryMissing(pid))
    }

    // -------------------------------------------------------------
    // Assertions
    // -------------------------------------------------------------

    pub(crate) fn assert_parent_exists(
        registry: &SubnetRegistrySnapshot,
        parent_pid: Principal,
    ) -> Result<(), Error> {
        if registry.entries.iter().any(|(pid, _)| *pid == parent_pid) {
            Ok(())
        } else {
            Err(TopologyPolicyError::ParentNotFound(parent_pid).into())
        }
    }

    pub(crate) fn assert_module_hash(
        registry: &SubnetRegistrySnapshot,
        pid: Principal,
        expected_hash: &[u8],
    ) -> Result<(), Error> {
        let record = Self::registry_record(registry, pid)?;

        if record.module_hash.as_deref() == Some(expected_hash) {
            Ok(())
        } else {
            Err(TopologyPolicyError::ModuleHashMismatch(pid).into())
        }
    }

    pub(crate) fn assert_immediate_parent(
        registry: &SubnetRegistrySnapshot,
        pid: Principal,
        expected_parent: Principal,
    ) -> Result<(), Error> {
        let record = Self::registry_record(registry, pid)?;

        match record.parent_pid {
            Some(pp) if pp == expected_parent => Ok(()),
            other => Err(TopologyPolicyError::ImmediateParentMismatch {
                pid,
                expected: expected_parent,
                found: other,
            }
            .into()),
        }
    }

    pub fn assert_directory_consistent_with_registry(
        registry: &SubnetRegistrySnapshot,
        entries: &[(CanisterRole, Principal)],
    ) -> Result<(), TopologyPolicyError> {
        let mut seen_roles = BTreeSet::new();

        for (role, pid) in entries {
            let record = Self::registry_record(registry, *pid)?;

            if record.role != *role {
                return Err(TopologyPolicyError::DirectoryRoleMismatch {
                    pid: *pid,
                    expected: record.role.clone(),
                    found: role.clone(),
                });
            }

            if !seen_roles.insert(role.clone()) {
                return Err(TopologyPolicyError::DuplicateDirectoryRole(role.clone()));
            }
        }

        Ok(())
    }
}
