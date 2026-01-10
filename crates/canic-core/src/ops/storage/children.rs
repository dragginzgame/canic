use crate::{
    ops::prelude::*,
    storage::{
        canister::CanisterRecord,
        stable::children::{CanisterChildren, CanisterChildrenData},
    },
};

///
/// ChildrenSnapshot
/// Internal snapshot of direct child canisters.
///
/// This is a cached projection populated via topology cascade.
/// Canonical derivation lives in `SubnetRegistry::children` /
/// `SubnetRegistryOps::children`.
/// Note: for non-root canisters, cached entries may have empty
/// `module_hash` / `created_at` fields; canonical data lives in the registry.
///
#[derive(Clone, Debug)]
pub struct ChildrenSnapshot {
    pub entries: Vec<(Principal, CanisterRecord)>,
}

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
        Self::snapshot().entries.iter().any(|(p, _)| p == pid)
    }

    #[must_use]
    pub fn pids() -> Vec<Principal> {
        Self::snapshot()
            .entries
            .into_iter()
            .map(|(pid, _)| pid)
            .collect()
    }

    // -------------------------------------------------------------
    // Snapshot / Import
    // -------------------------------------------------------------

    #[must_use]
    pub fn snapshot() -> ChildrenSnapshot {
        let data = CanisterChildren::export();
        ChildrenSnapshot {
            entries: data.entries,
        }
    }

    pub(crate) fn import(snapshot: ChildrenSnapshot) {
        CanisterChildren::import(CanisterChildrenData {
            entries: snapshot.entries,
        });
    }
}
