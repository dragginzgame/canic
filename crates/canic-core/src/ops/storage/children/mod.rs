//! Module: ops::storage::children
//!
//! Responsibility: expose deterministic local direct-child cache reads and imports.
//! Does not own: topology cascade workflow, root Component Registry truth, or endpoint DTOs.
//! Boundary: storage ops facade over child cache records.

use crate::{
    dto::{canister::CanisterInfo, component_registry::ComponentRuntimeDirectChild},
    ops::{prelude::*, storage::canister::record_to_info},
    storage::{
        canister::CanisterRecord,
        stable::children::{
            CanisterChildEntryRecord, CanisterChildRecord, CanisterChildren, CanisterChildrenData,
        },
    },
};

///
/// CanisterChildrenOps
///
/// Storage-ops facade for the direct-child cache.
///
/// Invariant: the children cache is replaced only from one validated directory projection.
///

pub struct CanisterChildrenOps;

impl CanisterChildrenOps {
    // -------------------------------------------------------------------------
    // Lookup helpers
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterRecord> {
        CanisterChildren::get(pid).map(|child| child.canister)
    }

    /// Return the exact Component allocation currently owning one direct child.
    #[must_use]
    pub fn allocation_operation_id(pid: Principal) -> Option<[u8; 32]> {
        CanisterChildren::get(pid)?.allocation_operation_id
    }

    #[must_use]
    pub fn matches_allocation(pid: Principal, operation_id: [u8; 32]) -> bool {
        Self::allocation_operation_id(pid) == Some(operation_id)
    }

    #[must_use]
    pub fn role_parent(pid: Principal) -> Option<(CanisterRole, Option<Principal>)> {
        Self::get(pid).map(|record| (record.role, record.parent_pid))
    }

    #[must_use]
    pub fn contains_pid(pid: &Principal) -> bool {
        CanisterChildren::get(*pid).is_some()
    }

    #[must_use]
    pub fn infos() -> Vec<CanisterInfo> {
        Self::records()
            .into_iter()
            .map(|entry| record_to_info(entry.pid, entry.record.canister))
            .collect()
    }

    #[must_use]
    fn records() -> Vec<CanisterChildEntryRecord> {
        Self::data().entries
    }

    #[must_use]
    pub fn pids() -> Vec<Principal> {
        Self::records().into_iter().map(|entry| entry.pid).collect()
    }

    // -------------------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn data() -> CanisterChildrenData {
        CanisterChildren::export()
    }

    pub(crate) fn import_topology_children(
        parent_pid: Principal,
        children: Vec<(Principal, CanisterRole)>,
    ) {
        // Cache entries omit module hash/created_at; canonical data lives at the Fleet Subnet
        // Root and reaches this canister through validated Directory synchronization.
        let data = CanisterChildrenData {
            entries: children
                .into_iter()
                .map(|(pid, role)| CanisterChildEntryRecord {
                    pid,
                    record: CanisterChildRecord {
                        allocation_operation_id: None,
                        canister: CanisterRecord {
                            role,
                            parent_pid: Some(parent_pid),
                            module_hash: None,
                            created_at: 0,
                        },
                    },
                })
                .collect(),
        };

        CanisterChildren::import(data);
    }

    /// Replace the local cache with one validated Root Directory projection.
    pub(crate) fn import_direct_children(
        parent_pid: Principal,
        children: Vec<ComponentRuntimeDirectChild>,
    ) {
        CanisterChildren::import(CanisterChildrenData {
            entries: children
                .into_iter()
                .map(|child| CanisterChildEntryRecord {
                    pid: child.canister_id,
                    record: CanisterChildRecord {
                        allocation_operation_id: Some(child.allocation_operation_id),
                        canister: CanisterRecord {
                            role: child.role,
                            parent_pid: Some(parent_pid),
                            module_hash: None,
                            created_at: 0,
                        },
                    },
                })
                .collect(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{
        seams::{lock, p},
        support::direct_child,
    };

    #[test]
    fn canonical_child_snapshot_retains_allocation_identity() {
        let _guard = lock();
        let pid = p(3);
        CanisterChildrenOps::import_direct_children(
            p(2),
            vec![direct_child(pid, CanisterRole::new("child"), [255; 32])],
        );
        let snapshot = CanisterChildrenOps::data();
        CanisterChildrenOps::import_direct_children(p(2), vec![]);
        CanisterChildren::import(snapshot);
        assert!(CanisterChildrenOps::matches_allocation(pid, [255; 32]));
        assert!(!CanisterChildrenOps::matches_allocation(pid, [1; 32]));
    }
}
