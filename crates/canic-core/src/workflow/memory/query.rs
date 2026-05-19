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
