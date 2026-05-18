use super::{
    ledger,
    registry::{MemoryRange, MemoryRangeAuthority, MemoryRegistryError},
};
use ic_memory::CommitStoreDiagnostic;

///
/// MemoryApi
///
/// Diagnostic facade for Canic-managed stable memory.

pub struct MemoryApi;

///
/// LedgerSnapshot
///
/// Read-only snapshot of the persisted ABI ledger.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerSnapshot {
    /// Ledger magic value from the physical header.
    pub magic: u64,
    /// Ledger physical format identifier from the header.
    pub format_id: u32,
    /// Ledger schema version from the header.
    pub schema_version: u32,
    /// Compiled layout epoch validated against the persisted header.
    pub layout_epoch: u32,
    /// Encoded ledger header length.
    pub header_len: u32,
    /// Header checksum covering the persisted header fields.
    pub header_checksum: u64,
    /// Authoritative committed generation selected by recovery validation.
    pub current_generation: u64,
    /// Protected commit slot recovery diagnostic.
    pub commit_recovery: CommitStoreDiagnostic,
    /// Canonical allocation authority ranges recorded by the persisted ABI ledger.
    pub authorities: Vec<MemoryRangeAuthority>,
    /// Historical owner ranges recorded by the persisted ABI ledger.
    pub ranges: Vec<(String, MemoryRange)>,
    /// Historical memory ID records recorded by the persisted ABI ledger.
    pub entries: Vec<(u8, super::registry::MemoryRegistryEntry)>,
}

impl MemoryApi {
    /// Read the persisted ABI ledger without relying on current registry reconstruction.
    pub fn ledger_snapshot() -> Result<LedgerSnapshot, MemoryRegistryError> {
        #[cfg(target_arch = "wasm32")]
        {
            let snapshot = ledger::try_diagnostic_snapshot()?;
            Ok(LedgerSnapshot::from(snapshot))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let snapshot = ledger::try_snapshot()?;
            Ok(LedgerSnapshot::from(snapshot))
        }
    }
}

impl From<ledger::MemoryLayoutLedgerSnapshot> for LedgerSnapshot {
    fn from(snapshot: ledger::MemoryLayoutLedgerSnapshot) -> Self {
        Self {
            magic: snapshot.magic,
            format_id: snapshot.format_id,
            schema_version: snapshot.schema_version,
            layout_epoch: snapshot.layout_epoch,
            header_len: snapshot.header_len,
            header_checksum: snapshot.header_checksum,
            current_generation: snapshot.current_generation,
            commit_recovery: snapshot.commit_recovery,
            authorities: snapshot.authorities,
            ranges: snapshot.ranges,
            entries: snapshot.entries,
        }
    }
}
