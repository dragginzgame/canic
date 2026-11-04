//! Project the single committed Storage runtime into protected diagnostic DTOs.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    dto::memory::{
        MemoryAllocationEntry, MemoryAllocationSizeEntry, MemoryAllocationsResponse,
        StorageDirectorySummary,
    },
    memory::{self, runtime::MemoryRuntimeAccessError},
};
use ic_storage::StorageDiagnostics;

/// Read-only access and synchronous bootstrap of the framework's Storage owner.
pub struct MemoryRegistryOps;

const fn invalid() -> InternalError {
    InternalError::public(crate::diagnostics::codes::STATE_INVALID)
}

impl From<MemoryRuntimeAccessError> for InternalError {
    fn from(_: MemoryRuntimeAccessError) -> Self {
        invalid()
    }
}

impl MemoryRegistryOps {
    /// Collect one validated directory view without constructing or opening allocations.
    pub fn allocation_snapshot() -> Result<MemoryAllocationsResponse, InternalError> {
        let (report, allocations) = memory::runtime::with_runtime(|runtime| {
            Ok((runtime.diagnostics()?, runtime.memory_allocations()?))
        })?;
        let summary = directory_summary(report)?;
        let memories = allocations
            .into_iter()
            .map(|entry| {
                Ok(MemoryAllocationEntry {
                    allocation_id: entry.id.get(),
                    stable_key: entry.stable_key.as_str().to_owned(),
                    capacity: MemoryAllocationSizeEntry {
                        wasm_pages: entry.pages,
                        bytes: entry
                            .pages
                            .checked_mul(ic_storage::WASM_PAGE_SIZE_BYTES)
                            .ok_or_else(invalid)?,
                    },
                })
            })
            .collect::<Result<Vec<_>, InternalError>>()?;
        Ok(MemoryAllocationsResponse { summary, memories })
    }

    /// Numeric upstream diagnostics; no per-key response is allocated.
    pub(crate) fn allocation_summary() -> Result<StorageDiagnostics, InternalError> {
        memory::runtime::with_runtime(|runtime| Ok(runtime.diagnostics()?)).map_err(Into::into)
    }

    pub(crate) fn init_registry() -> Result<(), InternalError> {
        memory::bootstrap_framework()
            .map_err(|_| InternalError::public(crate::diagnostics::codes::STORAGE_INVALID_STATE))
    }

    pub fn bootstrap_registry() -> Result<(), InternalError> {
        Self::init_registry()
    }

    pub fn is_initialized() -> Result<bool, InternalError> {
        memory::runtime::is_memory_bootstrap_ready().map_err(Into::into)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn ensure_bootstrap() -> Result<(), InternalError> {
        if Self::is_initialized()? {
            Ok(())
        } else {
            Self::bootstrap_registry()
        }
    }
}

fn directory_summary(report: StorageDiagnostics) -> Result<StorageDirectorySummary, InternalError> {
    Ok(StorageDirectorySummary {
        generation: report.generation,
        allocations: u64::try_from(report.allocations).map_err(|_| invalid())?,
        admitted_allocations: u64::try_from(report.admitted_allocations).map_err(|_| invalid())?,
        live_leases: u64::try_from(report.live_leases).map_err(|_| invalid())?,
        extents: u64::try_from(report.extents).map_err(|_| invalid())?,
        data_pages: report.data_pages,
        metadata_pages: report.metadata_pages,
        directory_bytes: report.directory_bytes,
        physical_pages: report.physical_pages,
        unassigned_pages: report.unassigned_pages,
    })
}
