//! Module: api::memory
//!
//! Responsibility: public memory-query facade for endpoint callers.
//! Does not own: memory accounting, stable-memory layout, or ledger updates.
//! Boundary: maps memory workflow errors into public API errors.

use crate::{
    dto::{error::Error, memory::MemoryAllocationsResponse},
    workflow::memory::query::MemoryQuery as MemoryQueryWorkflow,
};

///
/// MemoryQuery
///
/// Thin endpoint-facing facade for Storage directory queries.
///

pub struct MemoryQuery;

impl MemoryQuery {
    /// Return bounded current allocation extents without opening stable stores.
    pub fn allocations() -> Result<MemoryAllocationsResponse, Error> {
        MemoryQueryWorkflow::allocations().map_err(Error::from)
    }
}
