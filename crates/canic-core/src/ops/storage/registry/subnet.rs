//! Module: ops::storage::registry::subnet
//!
//! Responsibility: provide deterministic access to subnet canister registry records.
//! Does not own: stable registry schema, topology workflow, or endpoint DTOs.
//! Boundary: storage ops facade used by topology workflows and queries.

use crate::{
    InternalError,
    dto::topology::SubnetRegistryResponse,
    ops::{prelude::*, storage::StorageOpsError},
    storage::{
        canister::{CanisterEntryRecord, CanisterRecord},
        stable::registry::subnet::{SubnetRegistry, SubnetRegistryData},
    },
    view::topology::RegisteredCanisterView,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use thiserror::Error as ThisError;

///
/// SubnetRegistryOpsError
///
/// Typed storage failure for subnet registry mutation and parent-chain checks.
///

#[derive(Debug, ThisError)]
pub enum SubnetRegistryOpsError {
    #[error("canister {0} already registered")]
    AlreadyRegistered(Principal),

    #[error("canister {0} not found in subnet registry")]
    CanisterNotFound(Principal),

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

impl SubnetRegistryData {
    /// Return the canonical parent chain for a canister.
    ///
    /// Returned order: root → … → target
    pub(crate) fn parent_chain(
        &self,
        target: Principal,
    ) -> Result<Vec<CanisterEntryRecord>, InternalError> {
        let registry_len = self.entries.len();
        let index: HashMap<Principal, CanisterRecord> = self
            .entries
            .iter()
            .map(|entry| (entry.pid, entry.record.clone()))
            .collect();

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

            chain.push(CanisterEntryRecord {
                pid,
                record: record.clone(),
            });

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
/// Storage-ops facade for subnet canister registry records.
///

pub struct SubnetRegistryOps;

impl SubnetRegistryOps {
    // -------------------------------------------------------------------------
    // Mutation
    // -------------------------------------------------------------------------

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

    pub fn register_root_with_module_hash(
        pid: Principal,
        created_at: u64,
        module_hash: Option<Vec<u8>>,
    ) {
        SubnetRegistry::register_root_with_module_hash(pid, created_at, module_hash);
    }

    pub(crate) fn update_module_hash(pid: Principal, module_hash: Vec<u8>) -> bool {
        SubnetRegistry::update_module_hash(pid, module_hash)
    }

    pub(crate) fn unregister(pid: &Principal) -> bool {
        SubnetRegistry::remove(pid).is_some()
    }

    pub(crate) fn remove_and_return_role(pid: &Principal) -> Option<CanisterRole> {
        SubnetRegistry::remove(pid).map(|record| record.role)
    }

    // ---------------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------------

    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<CanisterRecord> {
        SubnetRegistry::get(pid)
    }

    /// Return the read-only registration metadata for one canister.
    #[must_use]
    pub fn registration(pid: Principal) -> Option<RegisteredCanisterView> {
        Self::get(pid).map(|record| RegisteredCanisterView {
            pid,
            created_at: record.created_at,
        })
    }

    #[must_use]
    pub fn role_parent(pid: Principal) -> Option<(CanisterRole, Option<Principal>)> {
        Self::get(pid).map(|record| (record.role, record.parent_pid))
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
    pub(crate) fn children(pid: Principal) -> Vec<CanisterEntryRecord> {
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
    ) -> Result<Vec<CanisterEntryRecord>, InternalError> {
        SubnetRegistry::export().parent_chain(target)
    }

    #[must_use]
    pub(crate) fn direct_children_map(
        parents: &[Principal],
    ) -> HashMap<Principal, Vec<CanisterEntryRecord>> {
        parents
            .iter()
            .map(|pid| (*pid, Self::children(*pid)))
            .collect()
    }

    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> SubnetRegistryData {
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

    /// Group direct root children by role for root-owned index validation.
    #[must_use]
    pub fn direct_root_role_index() -> BTreeMap<CanisterRole, Vec<Principal>> {
        let mut root_pid = None;
        SubnetRegistry::for_each(|pid, record| {
            if root_pid.is_none()
                && record.role == CanisterRole::ROOT
                && record.parent_pid.is_none()
            {
                root_pid = Some(pid);
            }
        });

        let Some(root_pid) = root_pid else {
            return BTreeMap::new();
        };

        let mut roles = BTreeMap::<CanisterRole, Vec<Principal>>::new();
        SubnetRegistry::for_each(|pid, record| {
            if record.parent_pid == Some(root_pid) {
                roles.entry(record.role).or_default().push(pid);
            }
        });
        for pids in roles.values_mut() {
            pids.sort();
        }
        roles
    }

    /// Resolve registration metadata for one role in deterministic canister-id order.
    #[must_use]
    pub fn registrations_for_role(role: &CanisterRole) -> Vec<RegisteredCanisterView> {
        let mut registrations = Vec::new();
        SubnetRegistry::for_each(|pid, record| {
            if &record.role == role {
                registrations.push(RegisteredCanisterView {
                    pid,
                    created_at: record.created_at,
                });
            }
        });
        registrations.sort_by_key(|registration| registration.pid);
        registrations
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn seed_registry() {
        let _ = SubnetRegistry::remove(&p(89));
        let _ = SubnetRegistry::remove(&p(90));
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
        SubnetRegistry::register(
            p(90),
            &CanisterRole::new("alpha_registry_test"),
            p(91),
            vec![4],
            4,
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

    #[test]
    fn registrations_for_role_returns_empty_for_absent_role() {
        seed_registry();

        assert!(
            SubnetRegistryOps::registrations_for_role(&CanisterRole::new("missing_registry_test"))
                .is_empty()
        );
    }

    #[test]
    fn registrations_for_role_preserves_creation_metadata() {
        seed_registry();

        assert_eq!(
            SubnetRegistryOps::registration(p(92)),
            Some(RegisteredCanisterView {
                pid: p(92),
                created_at: 2,
            })
        );
        assert_eq!(
            SubnetRegistryOps::registrations_for_role(&CanisterRole::new("alpha_registry_test")),
            vec![
                RegisteredCanisterView {
                    pid: p(90),
                    created_at: 4,
                },
                RegisteredCanisterView {
                    pid: p(92),
                    created_at: 2,
                },
            ]
        );
    }

    #[test]
    fn direct_root_role_index_excludes_nested_matching_roles() {
        seed_registry();
        SubnetRegistry::register(
            p(89),
            &CanisterRole::new("alpha_registry_test"),
            p(93),
            vec![5],
            5,
        );

        let roles = SubnetRegistryOps::direct_root_role_index();

        assert_eq!(
            roles.get(&CanisterRole::new("alpha_registry_test")),
            Some(&vec![p(90), p(92)])
        );
        assert_eq!(
            roles.get(&CanisterRole::new("beta_registry_test")),
            Some(&vec![p(93)])
        );
        assert!(!roles.values().flatten().any(|pid| pid == &p(89)));
    }
}
