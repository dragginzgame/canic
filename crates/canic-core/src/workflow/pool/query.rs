use crate::{
    dto::pool::{CanisterPoolEntryView, CanisterPoolView},
    ops::storage::pool::PoolOps,
    workflow::{pool::mapper::PoolMapper, prelude::*},
};

///
/// PoolQuery
///

pub struct PoolQuery;

impl PoolQuery {
    /// Return a view of a single pool entry (if present).
    pub fn pool_entry_view(pid: Principal) -> Option<CanisterPoolEntryView> {
        let data = PoolOps::data();

        data.entries
            .into_iter()
            .find(|(entry_pid, _)| *entry_pid == pid)
            .map(|(entry_pid, record)| PoolMapper::entry_data_to_view(entry_pid, record))
    }

    /// Return a view of the entire pool
    #[must_use]
    pub fn pool_list_view() -> CanisterPoolView {
        let data = PoolOps::data();
        PoolMapper::data_to_view(data)
    }
}
