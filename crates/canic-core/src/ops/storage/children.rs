use crate::{
    ops::{
        ic::IcOps, prelude::*, runtime::env::EnvOps, storage::registry::subnet::SubnetRegistryOps,
    },
    storage::{
        canister::CanisterRecord,
        stable::children::{CanisterChildren, CanisterChildrenData},
    },
};

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
    pub fn records() -> Vec<(Principal, CanisterRecord)> {
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
    pub fn data() -> CanisterChildrenData {
        CanisterChildren::export()
    }

    pub(crate) fn import(data: CanisterChildrenData) {
        CanisterChildren::import(data);
    }
}
