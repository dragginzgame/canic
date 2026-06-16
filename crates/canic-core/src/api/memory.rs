//! Module: api::memory
//!
//! Responsibility: public memory-query facade for endpoint callers.
//! Does not own: memory accounting, stable-memory layout, or ledger updates.
//! Boundary: maps memory workflow errors into public API errors.

use crate::{
    dto::{error::Error, memory::MemoryLedgerResponse},
    workflow::memory::query::MemoryQuery as MemoryQueryWorkflow,
};

///
/// MemoryQuery
///
/// Thin endpoint-facing facade for memory ledger queries.
///

pub struct MemoryQuery;

impl MemoryQuery {
    /// Return the current memory ledger snapshot.
    pub fn ledger() -> Result<MemoryLedgerResponse, Error> {
        MemoryQueryWorkflow::ledger().map_err(Error::from)
    }
}
