//! Module: memory::ledger
//!
//! Responsibility: adapt the native `ic-memory` runtime diagnostic export.
//! Does not own: memory-manager instances, ledger cells, allocation policy, or DTO shaping.
//! Boundary: diagnostics read the already bootstrapped Canic-owned runtime through `ic-memory`.

use ic_memory::{DiagnosticExport, MemoryAllocationPool, RuntimeDiagnosticError};

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
    pub commit_recovery: ic_memory::CommitSnapshotDiagnostic,
    pub allocation_pool: MemoryAllocationPool,
}

/// Read the committed allocation ledger from the Canic-owned runtime.
pub fn try_snapshot() -> Result<NativeMemoryLedgerSnapshot, MemoryLedgerError> {
    super::runtime::with_runtime(|runtime| {
        Ok(NativeMemoryLedgerSnapshot {
            export: runtime.diagnostic_export()?,
            commit_recovery: runtime.commit_recovery_diagnostic()?,
            allocation_pool: runtime
                .allocation_pool()
                .map_err(|_| RuntimeDiagnosticError::NotBootstrapped)?
                .clone(),
        })
    })
    .map_err(Into::into)
}

/// Failure to read the established runtime or its sealed host pool.
#[derive(Debug, thiserror::Error)]
pub enum MemoryLedgerError {
    #[error(transparent)]
    Diagnostic(#[from] RuntimeDiagnosticError),
}
