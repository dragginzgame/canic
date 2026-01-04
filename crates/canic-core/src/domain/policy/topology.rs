use crate::{
    Error, ThisError,
    cdk::types::Principal,
    domain::policy::PolicyError,
    ids::CanisterRole,
    ops::storage::{
        directory::{app::AppDirectorySnapshot, subnet::SubnetDirectorySnapshot},
        registry::subnet::{CanisterEntrySnapshot, SubnetRegistrySnapshot},
    },
};
use std::collections::BTreeMap;

///
/// TopologyPolicyError
///

#[derive(Debug, ThisError)]
pub enum TopologyPolicyError {
    #[error("parent {0} not found in registry")]
    ParentNotFound(Principal),

    #[error("registry entry missing for {0}")]
    RegistryEntryMissing(Principal),

    #[error("immediate-parent mismatch: canister {pid} expects parent {expected}, got {found:?}")]
    ImmediateParentMismatch {
        pid: Principal,
        expected: Principal,
        found: Option<Principal>,
    },

    #[error("module hash mismatch for {0}")]
    ModuleHashMismatch(Principal),

    #[error("app directory diverged from registry")]
    AppDirectoryDiverged,

    #[error("subnet directory diverged from registry")]
    SubnetDirectoryDiverged,
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
    pub(crate) fn registry_entry(
        registry: &SubnetRegistrySnapshot,
        pid: Principal,
    ) -> Result<CanisterEntrySnapshot, Error> {
        registry
            .entries
            .iter()
            .find(|(entry_pid, _)| *entry_pid == pid)
            .map(|(_, entry)| entry.clone())
            .ok_or_else(|| TopologyPolicyError::RegistryEntryMissing(pid).into())
    }

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
        expected_hash: Vec<u8>,
    ) -> Result<(), Error> {
        let entry = Self::registry_entry(registry, pid)?;

        if entry.module_hash == Some(expected_hash) {
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
        let entry = Self::registry_entry(registry, pid)?;

        match entry.parent_pid {
            Some(pp) if pp == expected_parent => Ok(()),
            other => Err(TopologyPolicyError::ImmediateParentMismatch {
                pid,
                expected: expected_parent,
                found: other,
            }
            .into()),
        }
    }

    #[must_use]
    pub fn app_directory_from_registry(registry: &SubnetRegistrySnapshot) -> AppDirectorySnapshot {
        let mut map = BTreeMap::<CanisterRole, Principal>::new();

        for (pid, entry) in &registry.entries {
            if crate::domain::policy::directory::is_app_directory_role(&entry.role) {
                map.insert(entry.role.clone(), *pid);
            }
        }

        AppDirectorySnapshot {
            entries: map.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn subnet_directory_from_registry(
        registry: &SubnetRegistrySnapshot,
    ) -> SubnetDirectorySnapshot {
        let mut map = BTreeMap::<CanisterRole, Principal>::new();

        for (pid, entry) in &registry.entries {
            if crate::domain::policy::directory::is_subnet_directory_role(&entry.role) {
                map.insert(entry.role.clone(), *pid);
            }
        }

        SubnetDirectorySnapshot {
            entries: map.into_iter().collect(),
        }
    }

    pub fn assert_directories_match_registry(
        registry: &SubnetRegistrySnapshot,
        app_snapshot: &AppDirectorySnapshot,
        subnet_snapshot: &SubnetDirectorySnapshot,
    ) -> Result<(), TopologyPolicyError> {
        let app_built = Self::app_directory_from_registry(registry);
        if app_built != *app_snapshot {
            return Err(TopologyPolicyError::AppDirectoryDiverged);
        }

        let subnet_built = Self::subnet_directory_from_registry(registry);
        if subnet_built != *subnet_snapshot {
            return Err(TopologyPolicyError::SubnetDirectoryDiverged);
        }

        Ok(())
    }
}
