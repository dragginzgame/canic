use crate::{
    cdk::types::Principal,
    ids::CanisterRole,
    storage::memory::children::{CanisterChildren, CanisterChildrenData},
};

///
/// ChildSnapshot
/// Internal, operational snapshot of a child canister.
///

pub struct ChildSnapshot {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}

///
/// ChildrenSnapshot
/// Internal snapshot of direct children.
/// Projection of cached children; canonical derivation is
/// `SubnetRegistry::children` / `SubnetRegistryOps::children`.
///

pub struct ChildrenSnapshot {
    pub entries: Vec<ChildSnapshot>,
}

///
/// CanisterChildrenOps
///
/// Invariant: the children cache is updated only via topology cascade
/// (workflow::cascade::topology::nonroot_cascade_topology).

pub struct CanisterChildrenOps;

impl CanisterChildrenOps {
    // -------------------------------------------------------------
    // Lookup helpers (internal)
    // -------------------------------------------------------------

    #[must_use]
    pub fn contains_pid(pid: &Principal) -> bool {
        Self::snapshot().entries.iter().any(|e| &e.pid == pid)
    }

    #[must_use]
    pub fn find_by_pid(pid: &Principal) -> Option<ChildSnapshot> {
        Self::snapshot().entries.into_iter().find(|e| &e.pid == pid)
    }

    #[must_use]
    pub fn find_first_by_role(role: &CanisterRole) -> Option<ChildSnapshot> {
        Self::snapshot()
            .entries
            .into_iter()
            .find(|e| &e.role == role)
    }

    #[must_use]
    pub fn pids() -> Vec<Principal> {
        Self::snapshot()
            .entries
            .into_iter()
            .map(|e| e.pid)
            .collect()
    }

    // -------------------------------------------------------------
    // Import / Snapshot
    // -------------------------------------------------------------

    #[must_use]
    pub fn snapshot() -> ChildrenSnapshot {
        let data = CanisterChildren::export();

        ChildrenSnapshot {
            entries: data
                .entries
                .into_iter()
                .map(|(pid, summary)| ChildSnapshot {
                    pid,
                    role: summary.role,
                    parent_pid: summary.parent_pid,
                })
                .collect(),
        }
    }

    pub(crate) fn import(snapshot: ChildrenSnapshot) {
        let entries = snapshot
            .entries
            .into_iter()
            .map(|e| {
                (
                    e.pid,
                    crate::storage::canister::CanisterSummary {
                        role: e.role,
                        parent_pid: e.parent_pid,
                    },
                )
            })
            .collect();

        CanisterChildren::import(CanisterChildrenData { entries });
    }
}
