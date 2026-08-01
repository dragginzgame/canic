//! Module: ops::storage::children
//!
//! Responsibility: expose deterministic local direct-child cache reads and imports.
//! Does not own: topology cascade workflow, root Component Registry truth, or endpoint DTOs.
//! Boundary: storage ops facade over child cache records.

use crate::{
    dto::canister::CanisterInfo,
    ops::{prelude::*, storage::canister::record_to_info},
    storage::{
        canister::{CanisterEntryRecord, CanisterRecord},
        stable::children::{CanisterChildren, CanisterChildrenData},
    },
};

///
/// CanisterChildrenOps
///
/// Storage-ops facade for the direct-child cache.
///
/// Invariant: the children cache is updated only via the topology cascade workflow.
///

pub struct CanisterChildrenOps;

impl CanisterChildrenOps {
    // -------------------------------------------------------------------------
    // Lookup helpers
    // -------------------------------------------------------------------------

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterRecord> {
        CanisterChildren::get(pid)
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
            .map(|entry| record_to_info(entry.pid, entry.record))
            .collect()
    }

    #[must_use]
    fn records() -> Vec<CanisterEntryRecord> {
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

    pub(crate) fn import_direct_children(
        parent_pid: Principal,
        children: Vec<(Principal, CanisterRole)>,
    ) {
        // Cache entries omit module hash/created_at; canonical data lives at the Fleet Subnet
        // Root and reaches this canister through the topology cascade.
        let data = CanisterChildrenData {
            entries: children
                .into_iter()
                .map(|(pid, role)| CanisterEntryRecord {
                    pid,
                    record: CanisterRecord {
                        role,
                        parent_pid: Some(parent_pid),
                        module_hash: None,
                        created_at: 0,
                    },
                })
                .collect(),
        };

        CanisterChildren::import(data);
    }
}
