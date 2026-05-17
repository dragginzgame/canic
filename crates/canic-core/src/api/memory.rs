use crate::{
    dto::{
        error::Error,
        memory::{MemoryLedgerResponse, MemoryRegistryResponse},
    },
    workflow::memory::query::MemoryQuery as MemoryQueryWorkflow,
};

///
/// MemoryQuery
///

pub struct MemoryQuery;

impl MemoryQuery {
    #[must_use]
    pub fn registry() -> MemoryRegistryResponse {
        MemoryQueryWorkflow::registry()
    }

    pub fn ledger() -> Result<MemoryLedgerResponse, Error> {
        MemoryQueryWorkflow::ledger().map_err(Error::from)
    }
}
