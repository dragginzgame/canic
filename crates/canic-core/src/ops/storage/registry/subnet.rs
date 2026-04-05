use crate::{
    InternalError,
    dto::topology::SubnetRegistryResponse,
    ops::{prelude::*, storage::StorageOpsError},
    storage::{
        canister::CanisterRecord,
        stable::registry::subnet::{SubnetRegistry, SubnetRegistryRecord},
    },
};
use std::collections::{BTreeMap, HashMap, HashSet};
use thiserror::Error as ThisError;

///
/// SubnetRegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum SubnetRegistryOpsError {
    #[error("canister {0} already registered")]
    AlreadyRegistered(Principal),

    #[error("canister {0} not found in subnet registry")]
    CanisterNotFound(Principal),

    #[error("role '{0}' not found in subnet registry")]
    RoleNotFound(CanisterRole),

    #[error("parent chain contains a cycle at {0}")]
    ParentChainCycle(Principal),

    #[error("parent chain exceeded registry size ({0})")]
    ParentChainTooLong(usize),

    #[error("parent chain did not terminate at root (last pid: {0})")]
    ParentChainNotRootTerminated(Principal),

    #[error("parent canister {0} not found in subnet registry")]
    ParentNotFound(Principal),
}

impl From<SubnetRegistryOpsError> for InternalError {
    fn from(err: SubnetRegistryOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

impl SubnetRegistryRecord {
    /// Return the canonical parent chain for a canister.
    ///
    /// Returned order: root → … → target
    pub(crate) fn parent_chain(
        &self,
        target: Principal,
    ) -> Result<Vec<(Principal, CanisterRecord)>, InternalError> {
        let registry_len = self.entries.len();
        let index: HashMap<Principal, CanisterRecord> = self.entries.iter().cloned().collect();

        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        let mut pid = target;

        loop {
            if !seen.insert(pid) {
                return Err(SubnetRegistryOpsError::ParentChainCycle(pid).into());
            }

            let record = index
                .get(&pid)
                .ok_or(SubnetRegistryOpsError::CanisterNotFound(pid))?;

            if seen.len() > registry_len {
                return Err(SubnetRegistryOpsError::ParentChainTooLong(seen.len()).into());
            }

            chain.push((pid, record.clone()));

            if let Some(parent_pid) = record.parent_pid {
                pid = parent_pid;
            } else {
                if record.role != CanisterRole::ROOT {
                    return Err(SubnetRegistryOpsError::ParentChainNotRootTerminated(pid).into());
                }

                break;
            }
        }

        chain.reverse();
        Ok(chain)
    }
}

///
/// SubnetRegistryOps
///

pub struct SubnetRegistryOps;

impl SubnetRegistryOps {
    // ---------------------------------------------------------------------
    // Mutation
    // ---------------------------------------------------------------------

    pub fn register_unchecked(
        pid: Principal,
        role: &CanisterRole,
        parent_pid: Principal,
        module_hash: Vec<u8>,
        created_at: u64,
    ) -> Result<(), InternalError> {
        if SubnetRegistry::get(pid).is_some() {
            return Err(SubnetRegistryOpsError::AlreadyRegistered(pid).into());
        }

        if SubnetRegistry::get(parent_pid).is_none() {
            return Err(SubnetRegistryOpsError::ParentNotFound(parent_pid).into());
        }

        SubnetRegistry::register(pid, role, parent_pid, module_hash, created_at);
        Ok(())
    }

    pub fn register_root(pid: Principal, created_at: u64) {
        SubnetRegistry::register_root(pid, created_at);
    }

    pub(crate) fn update_module_hash(pid: Principal, module_hash: Vec<u8>) -> bool {
        SubnetRegistry::update_module_hash(pid, module_hash)
    }

    pub(crate) fn remove(pid: &Principal) -> Option<CanisterRecord> {
        SubnetRegistry::remove(pid)
    }

    // ---------------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------------

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterRecord> {
        SubnetRegistry::get(pid)
    }

    #[must_use]
    pub(crate) fn is_registered(pid: Principal) -> bool {
        SubnetRegistry::get(pid).is_some()
    }

    #[must_use]
    pub fn has_role(role: &CanisterRole) -> bool {
        Self::find_pid_for_role(role).is_some()
    }

    #[must_use]
    pub fn get_parent(pid: Principal) -> Option<Principal> {
        SubnetRegistry::get_parent(pid)
    }

    /// Direct children (one level).
    #[must_use]
    pub(crate) fn children(pid: Principal) -> Vec<(Principal, CanisterRecord)> {
        SubnetRegistry::children(pid)
    }

    #[must_use]
    pub(crate) fn find_pid_for_role(role: &CanisterRole) -> Option<Principal> {
        SubnetRegistry::find_pid_for_role(role)
    }

    #[must_use]
    pub(crate) fn find_child_pid_for_role(
        parent: Principal,
        role: &CanisterRole,
    ) -> Option<Principal> {
        SubnetRegistry::find_child_pid_for_role(parent, role)
    }

    pub(crate) fn parent_chain(
        target: Principal,
    ) -> Result<Vec<(Principal, CanisterRecord)>, InternalError> {
        SubnetRegistry::export().parent_chain(target)
    }

    #[must_use]
    pub(crate) fn direct_children_map(
        parents: &[Principal],
    ) -> HashMap<Principal, Vec<(Principal, CanisterRecord)>> {
        parents
            .iter()
            .map(|pid| (*pid, Self::children(*pid)))
            .collect()
    }

    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> SubnetRegistryRecord {
        SubnetRegistry::export()
    }

    #[must_use]
    pub fn response() -> SubnetRegistryResponse {
        let mut entries = Vec::with_capacity(SubnetRegistry::len());

        SubnetRegistry::for_each(|pid, record| {
            entries
                .push(super::mapper::SubnetRegistryResponseMapper::record_to_response(pid, record));
        });

        SubnetRegistryResponse(entries)
    }

    #[must_use]
    pub fn role_index() -> BTreeMap<CanisterRole, Vec<Principal>> {
        let mut roles = BTreeMap::<CanisterRole, Vec<Principal>>::new();

        SubnetRegistry::for_each(|pid, entry| {
            roles.entry(entry.role).or_default().push(pid);
        });

        roles
    }

    /// Resolve all registered canister ids for one role in deterministic order.
    pub fn pids_for_role(role: &CanisterRole) -> Result<Vec<Principal>, InternalError> {
        let mut pids = Self::role_index()
            .remove(role)
            .ok_or_else(|| SubnetRegistryOpsError::RoleNotFound(role.clone()))?;
        pids.sort();
        Ok(pids)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn seed_registry() {
        let _ = SubnetRegistry::remove(&p(93));
        let _ = SubnetRegistry::remove(&p(92));
        let _ = SubnetRegistry::remove(&p(91));

        SubnetRegistry::register_root(p(91), 1);
        SubnetRegistry::register(
            p(92),
            &CanisterRole::new("alpha_registry_test"),
            p(91),
            vec![2],
            2,
        );
        SubnetRegistry::register(
            p(93),
            &CanisterRole::new("beta_registry_test"),
            p(91),
            vec![3],
            3,
        );
    }

    #[test]
    fn response_builds_registry_view_without_export_snapshot() {
        seed_registry();

        let response = SubnetRegistryOps::response();
        let alpha = response
            .0
            .iter()
            .find(|entry| entry.pid == p(92))
            .expect("alpha entry present");
        let beta = response
            .0
            .iter()
            .find(|entry| entry.pid == p(93))
            .expect("beta entry present");

        assert!(response.0.iter().any(|entry| entry.pid == p(91)));
        assert_eq!(alpha.role, CanisterRole::new("alpha_registry_test"));
        assert_eq!(alpha.record.parent_pid, Some(p(91)));
        assert_eq!(beta.record.module_hash, Some(vec![3]));
    }
}
