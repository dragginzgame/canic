//! Module: memory::ledger
//!
//! Responsibility: adapt the native `ic-memory` runtime diagnostic export.
//! Does not own: memory-manager instances, ledger cells, allocation policy, or DTO shaping.
//! Boundary: diagnostics read the already bootstrapped Canic-owned runtime through `ic-memory`.

use crate::memory::{pool, registry::MemoryRegistryError};
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
    Ok(NativeMemoryLedgerSnapshot {
        export: super::runtime::with_runtime(ic_memory::MemoryRuntime::diagnostic_export)?,
        commit_recovery: super::runtime::with_runtime(
            ic_memory::MemoryRuntime::commit_recovery_diagnostic,
        )?,
        allocation_pool: pool::selected()?,
    })
}

/// Failure to read the established runtime or its sealed host pool.
#[derive(Debug, thiserror::Error)]
pub enum MemoryLedgerError {
    #[error(transparent)]
    Diagnostic(#[from] RuntimeDiagnosticError),
    #[error(transparent)]
    Pool(#[from] MemoryRegistryError),
}
