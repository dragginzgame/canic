//! Module: workflow::pool::query
//!
//! Responsibility: expose read-only pool workflow queries.
//! Does not own: pool storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over pool storage ops.

use crate::{
    cdk::types::Principal,
    dto::pool::{CanisterPoolEntry, CanisterPoolResponse},
    ops::storage::pool::{
        PoolOps,
        mapper::{CanisterPoolEntryMapper, CanisterPoolResponseMapper},
    },
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
            .find(|entry| entry.pid == pid)
            .map(|entry| CanisterPoolEntryMapper::record_to_view(entry.pid, entry.record))
    }

    /// Return a view of the entire pool
    #[must_use]
    pub fn pool_list() -> CanisterPoolResponse {
        let data = PoolOps::data();
        CanisterPoolResponseMapper::data_to_view(data)
    }
}
