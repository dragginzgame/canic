pub mod registry;

use crate::{
    domain::value::Principal,
    ids::CanisterRole,
    model::topology::{TopologyDirectoryEntry, TopologyEntry, TopologyRegistry},
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

///
/// TopologyPolicyError
///

#[derive(Debug, ThisError)]
pub enum TopologyPolicyError {
    #[error("Directory entry role mismatch for pid {pid}: expected {expected}, got {found}")]
    DirectoryRoleMismatch {
        pid: Principal,
        expected: CanisterRole,
        found: CanisterRole,
    },

    #[error("Directory role {0} appears more than once")]
    DuplicateDirectoryRole(CanisterRole),

    #[error("immediate-parent mismatch: canister {pid} expects parent {expected}, got {found:?}")]
    ImmediateParentMismatch {
        pid: Principal,
        expected: Principal,
        found: Option<Principal>,
    },

    #[error("parent {0} not found in registry")]
    ParentNotFound(Principal),

    #[error("registry entry missing for {0}")]
    RegistryEntryMissing(Principal),

    #[error(transparent)]
    RegistryPolicy(#[from] registry::RegistryPolicyError),
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
        registry: &'_ TopologyRegistry,
        pid: Principal,
    ) -> Result<&'_ TopologyEntry, TopologyPolicyError> {
        registry
            .entries
            .iter()
            .find(|entry| entry.pid == pid)
            .ok_or(TopologyPolicyError::RegistryEntryMissing(pid))
    }

    pub fn assert_directory_consistent_with_registry(
        registry: &TopologyRegistry,
        entries: &[TopologyDirectoryEntry],
    ) -> Result<(), TopologyPolicyError> {
        let mut seen_roles = BTreeSet::new();

        for entry in entries {
            let record = Self::registry_record(registry, entry.pid)?;

            if record.role != entry.role {
                return Err(TopologyPolicyError::DirectoryRoleMismatch {
                    pid: entry.pid,
                    expected: record.role.clone(),
                    found: entry.role.clone(),
                });
            }

            if !seen_roles.insert(entry.role.clone()) {
                return Err(TopologyPolicyError::DuplicateDirectoryRole(
                    entry.role.clone(),
                ));
            }
        }

        Ok(())
    }
}
