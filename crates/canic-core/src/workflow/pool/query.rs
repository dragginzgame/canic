use crate::{
    dto::pool::{CanisterPoolEntryView, CanisterPoolView},
    ops::storage::pool::PoolOps,
    workflow::{pool::mapper::PoolMapper, prelude::*},
};

/// Return a view of a single pool entry (if present).
pub fn pool_entry_view(pid: Principal) -> Option<CanisterPoolEntryView> {
    let snapshot = PoolOps::snapshot();

    snapshot
        .entries
        .into_iter()
        .find(|e| e.pid == pid)
        .map(PoolMapper::entry_snapshot_to_view)
}

/// Return a view of the entire pool
#[must_use]
pub fn pool_list_view() -> CanisterPoolView {
    let snapshot = PoolOps::snapshot();
    PoolMapper::snapshot_to_view(snapshot)
}
