//! Module: memory::ledger
//!
//! Responsibility: adapt the native `ic-memory` runtime diagnostic export.
//! Does not own: memory-manager instances, ledger cells, allocation policy, or DTO shaping.
//! Boundary: diagnostics read the already bootstrapped default runtime through `ic-memory`.

use super::policy;
use ic_memory::{DiagnosticExport, MemoryManagerAuthorityRecord, RuntimeDiagnosticError};

pub const MEMORY_LEDGER_SCHEMA_VERSION: u32 = 1;
pub const MEMORY_PHYSICAL_FORMAT_ID: u32 = 1;

///
/// NativeMemoryLedgerSnapshot
///
/// Diagnostic snapshot of the native memory allocation ledger and authorities.
/// Owned by memory ledger and consumed by diagnostics/status surfaces.
///

pub struct NativeMemoryLedgerSnapshot {
    pub export: DiagnosticExport,
    pub authorities: Vec<MemoryManagerAuthorityRecord>,
}

/// Read the committed allocation ledger from the canonical default runtime.
pub fn try_snapshot() -> Result<NativeMemoryLedgerSnapshot, RuntimeDiagnosticError> {
    Ok(NativeMemoryLedgerSnapshot {
        export: ic_memory::default_memory_manager_diagnostic_export()?,
        authorities: policy::canonical_authority_records(),
    })
}
