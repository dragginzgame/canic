pub mod registry;

use crate::{
    InternalError,
    domain::value::Principal,
    ids::CanisterRole,
    model::topology::{TopologyEntry, TopologyIndexEntry, TopologyRegistry},
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

///
/// TopologyPolicyError
///

#[derive(Debug, ThisError)]
pub enum TopologyPolicyError {
    #[error("index entry role mismatch for pid {pid}: expected {expected}, got {found}")]
    IndexRoleMismatch {
        pid: Principal,
        expected: CanisterRole,
        found: CanisterRole,
    },

    #[error("index role {0} appears more than once")]
    DuplicateIndexRole(CanisterRole),

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

    // -------------------------------------------------------------
    // Assertions
    // -------------------------------------------------------------

    pub(crate) fn assert_parent_exists(
        registry: &TopologyRegistry,
        parent_pid: Principal,
    ) -> Result<(), InternalError> {
        if registry.entries.iter().any(|entry| entry.pid == parent_pid) {
            Ok(())
        } else {
            Err(TopologyPolicyError::ParentNotFound(parent_pid).into())
        }
    }

    pub(crate) fn assert_module_hash(
        registry: &TopologyRegistry,
        pid: Principal,
        expected_hash: &[u8],
    ) -> Result<(), InternalError> {
        let record = Self::registry_record(registry, pid)?;

        if record.module_hash.as_deref() == Some(expected_hash) {
            Ok(())
        } else {
            Err(TopologyPolicyError::ModuleHashMismatch(pid).into())
        }
    }

    pub(crate) fn assert_immediate_parent(
        registry: &TopologyRegistry,
        pid: Principal,
        expected_parent: Principal,
    ) -> Result<(), InternalError> {
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

    pub fn assert_index_consistent_with_registry(
        registry: &TopologyRegistry,
        entries: &[TopologyIndexEntry],
    ) -> Result<(), TopologyPolicyError> {
        let mut seen_roles = BTreeSet::new();

        for entry in entries {
            let record = Self::registry_record(registry, entry.pid)?;

            if record.role != entry.role {
                return Err(TopologyPolicyError::IndexRoleMismatch {
                    pid: entry.pid,
                    expected: record.role.clone(),
                    found: entry.role.clone(),
                });
            }

            if !seen_roles.insert(entry.role.clone()) {
                return Err(TopologyPolicyError::DuplicateIndexRole(entry.role.clone()));
            }
        }

        Ok(())
    }
}
