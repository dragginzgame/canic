//! Module: workflow::pool::query
//!
//! Responsibility: expose read-only pool workflow queries.
//! Does not own: pool storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over pool storage ops.

use crate::{
    dto::pool::{CanisterPoolEntry, CanisterPoolResponse},
    ops::storage::pool::{
        PoolOps,
        mapper::{CanisterPoolEntryMapper, CanisterPoolResponseMapper},
    },
    workflow::prelude::*,
};

///
/// PoolQuery
///

pub struct PoolQuery;

impl PoolQuery {
    /// Return a view of a single pool entry (if present).
    #[must_use]
    pub fn pool_entry(pid: Principal) -> Option<CanisterPoolEntry> {
        let data = PoolOps::data();

        data.entries
            .into_iter()
            .find(|(entry_pid, _)| *entry_pid == pid)
            .map(|(entry_pid, record)| CanisterPoolEntryMapper::record_to_view(entry_pid, record))
    }

    /// Return a view of the entire pool
    #[must_use]
    pub fn pool_list() -> CanisterPoolResponse {
        let data = PoolOps::data();
        CanisterPoolResponseMapper::record_to_view(data)
    }
}
