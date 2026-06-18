//! Module: workflow::memory::query
//!
//! Responsibility: expose memory ledger workflow snapshots.
//! Does not own: memory registry mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over runtime memory ops.

use crate::{
    InternalError, dto::memory::MemoryLedgerResponse, ops::runtime::memory::MemoryRegistryOps,
};

///
/// MemoryQuery
///

pub struct MemoryQuery;

impl MemoryQuery {
    pub fn ledger() -> Result<MemoryLedgerResponse, InternalError> {
        MemoryRegistryOps::ledger_snapshot()
    }
}
