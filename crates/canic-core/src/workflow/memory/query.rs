use crate::{
    InternalError,
    dto::memory::{MemoryLedgerResponse, MemoryRegistryResponse},
    ops::runtime::memory::MemoryRegistryOps,
};

///
/// MemoryQuery
///

pub struct MemoryQuery;

impl MemoryQuery {
    #[must_use]
    pub fn registry() -> MemoryRegistryResponse {
        let entries = MemoryRegistryOps::snapshot_entries();
        MemoryRegistryResponse { entries }
    }

    pub fn ledger() -> Result<MemoryLedgerResponse, InternalError> {
        MemoryRegistryOps::ledger_snapshot()
    }
}
