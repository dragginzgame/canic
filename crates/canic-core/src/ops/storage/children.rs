use crate::{
    ops::prelude::*,
    storage::stable::children::{CanisterChildren, CanisterChildrenData},
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
        Self::data().entries.iter().any(|(p, _)| p == pid)
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
