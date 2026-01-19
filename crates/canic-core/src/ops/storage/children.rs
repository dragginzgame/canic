mod mapper;

use crate::{
    dto::canister::CanisterInfo,
    ops::{
        ic::IcOps, prelude::*, runtime::env::EnvOps, storage::registry::subnet::SubnetRegistryOps,
    },
    storage::{
        canister::CanisterRecord,
        stable::children::{CanisterChildren, CanisterChildrenRecord},
    },
};
use mapper::CanisterRecordMapper;

///
/// CanisterChildrenOps
///
/// Invariant: the children cache is updated only via topology cascade
/// (workflow::cascade::topology::nonroot_cascade_topology).
///
pub struct CanisterChildrenOps;

impl CanisterChildrenOps {
    // -------------------------------------------------------------
    // Lookup helpers
    // -------------------------------------------------------------

    #[must_use]
    pub fn contains_pid(pid: &Principal) -> bool {
        if EnvOps::is_root() {
            SubnetRegistryOps::children(IcOps::canister_self())
                .iter()
                .any(|(child_pid, _)| child_pid == pid)
        } else {
            Self::data().entries.iter().any(|(p, _)| p == pid)
        }
    }

    #[must_use]
    pub fn infos() -> Vec<CanisterInfo> {
        Self::records()
            .into_iter()
            .map(|(pid, record)| CanisterRecordMapper::record_to_response(pid, record))
            .collect()
    }

    #[must_use]
    fn records() -> Vec<(Principal, CanisterRecord)> {
        if EnvOps::is_root() {
            SubnetRegistryOps::children(IcOps::canister_self())
        } else {
            Self::data().entries
        }
    }

    #[must_use]
    pub fn pids() -> Vec<Principal> {
        Self::data()
            .entries
            .into_iter()
            .map(|(pid, _)| pid)
            .collect()
    }

    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> CanisterChildrenRecord {
        CanisterChildren::export()
    }

    #[allow(dead_code)]
    pub(crate) fn import(data: CanisterChildrenRecord) {
        CanisterChildren::import(data);
    }

    pub(crate) fn import_direct_children(
        parent_pid: Principal,
        children: Vec<(Principal, CanisterRole)>,
    ) {
        // Cache entries omit module hash/created_at; canonical data lives in the registry.
        let data = CanisterChildrenRecord {
            entries: children
                .into_iter()
                .map(|(pid, role)| {
                    (
                        pid,
                        CanisterRecord {
                            role,
                            parent_pid: Some(parent_pid),
                            module_hash: None,
                            created_at: 0,
                        },
                    )
                })
                .collect(),
        };

        CanisterChildren::import(data);
    }
}
