//! Module: ops::runtime::memory
//!
//! Responsibility: commit memory allocation declarations and expose memory diagnostics.
//! Does not own: memory schema declarations, stable records, or DTO schema.
//! Boundary: maps memory runtime diagnostics into ops query responses.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    domain::memory::{
        MemoryAllocationBinding, MemoryAllocationState, MemoryCommitRecoveryErrorResponse,
    },
    dto::memory::{
        MemoryAllocationEntry, MemoryAllocationPoolEntry, MemoryAllocationRecordEntry,
        MemoryAllocationSizeEntry, MemoryAllocationsResponse, MemoryAuthorityEntry,
        MemoryCommitRecoveryResponse, MemoryCommitSnapshotResponse, MemoryExcludedRangeEntry,
        MemoryLedgerMemoryEntry, MemoryLedgerResponse,
    },
    memory::{self, ledger, registry::MemoryRegistryError},
};
use ic_memory::{
    AllocationState, CommitSnapshotDiagnostic, DiagnosticMemorySize, DiagnosticRecord,
};
use thiserror::Error as ThisError;

///
/// MemoryRegistryOpsError
///
/// Typed failure surface for memory registry bootstrap and diagnostics.
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryOpsError {
    #[error(transparent)]
    Ledger(#[from] ledger::MemoryLedgerError),
    // this error comes from the Canic memory runtime boundary
    #[error(transparent)]
    Registry(#[from] MemoryRegistryError),
    // this error comes from the generic ic-memory runtime boundary
    #[error(transparent)]
    Runtime(#[from] memory::MemoryBootstrapError),
    // this error comes from the generic ic-memory runtime diagnostic boundary
    #[error(transparent)]
    Diagnostic(#[from] ic_memory::RuntimeDiagnosticError),
}

impl From<MemoryRegistryOpsError> for InternalError {
    fn from(err: MemoryRegistryOpsError) -> Self {
        let code = match err {
            MemoryRegistryOpsError::Registry(_)
            | MemoryRegistryOpsError::Runtime(_)
            | MemoryRegistryOpsError::Ledger(ledger::MemoryLedgerError::Pool(_)) => {
                crate::diagnostics::codes::STORAGE_INVALID_STATE
            }
            MemoryRegistryOpsError::Diagnostic(_)
            | MemoryRegistryOpsError::Ledger(ledger::MemoryLedgerError::Diagnostic(_)) => {
                crate::diagnostics::codes::STATE_INVALID
            }
        };
        Self::public(code)
    }
}

///
/// MemoryRegistryOps
///
/// Operations-layer facade for memory registry bootstrap and diagnostics.
///

pub struct MemoryRegistryOps;

impl MemoryRegistryOps {
    /// Measure all usable IDs through the substrate's bounded read-only report.
    /// Collection never decodes history or constructs a missing runtime.
    pub fn allocation_snapshot() -> Result<MemoryAllocationsResponse, InternalError> {
        memory_allocations_response(Self::allocation_report()?)
    }

    /// Share the bounded owner report without constructing protected response rows.
    pub(crate) fn allocation_report() -> Result<ic_memory::MemoryAllocations, InternalError> {
        memory::runtime::with_runtime(ic_memory::MemoryRuntime::memory_allocations)
            .map_err(MemoryRegistryOpsError::from)
            .map_err(Into::into)
    }

    /// Collect numeric allocation totals without constructing per-ID rows or copying names.
    pub(crate) fn allocation_summary() -> Result<ic_memory::MemoryAllocationSummary, InternalError>
    {
        memory::runtime::with_runtime(ic_memory::MemoryRuntime::memory_allocation_summary)
            .map_err(MemoryRegistryOpsError::from)
            .map_err(Into::into)
    }

    // Initialize the stable-memory registry for this crate and summarize the layout.
    pub(crate) fn init_registry() -> Result<(), InternalError> {
        memory::bootstrap_framework().map_err(MemoryRegistryOpsError::from)?;
        Ok(())
    }

    // Run the full synchronous Canic memory bootstrap and return the committed layout.
    pub fn bootstrap_registry() -> Result<(), InternalError> {
        Self::init_registry()
    }

    pub fn is_initialized() -> Result<bool, InternalError> {
        crate::memory::runtime::is_memory_bootstrap_ready()
            .map_err(MemoryRegistryOpsError::from)
            .map_err(Into::into)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn ensure_bootstrap() -> Result<(), InternalError> {
        if Self::is_initialized()? {
            return Ok(());
        }

        Self::bootstrap_registry()
    }

    // Read the committed ABI ledger using the restricted diagnostic path.
    pub fn ledger_snapshot() -> Result<MemoryLedgerResponse, InternalError> {
        let snapshot = ledger::try_snapshot().map_err(MemoryRegistryOpsError::from)?;

        let allocation_pool = memory_allocation_pool_response(&snapshot.allocation_pool);

        let records: Vec<MemoryAllocationRecordEntry> = snapshot
            .export
            .records
            .into_iter()
            .map(memory_allocation_record_response)
            .collect();
        let memories = memory_ledger_memory_entries(&records);
        Ok(MemoryLedgerResponse {
            ledger_schema_version: crate::memory::ledger::MEMORY_LEDGER_SCHEMA_VERSION,
            physical_format_id: crate::memory::ledger::MEMORY_PHYSICAL_FORMAT_ID,
            current_generation: snapshot.export.current_generation,
            ledger_memory_manager_id: snapshot.export.ledger_anchor.id(),
            commit_recovery: commit_recovery_response(snapshot.export.commit_recovery),
            allocation_pool,
            memories,
            records,
        })
    }
}

fn memory_allocations_response(
    report: ic_memory::MemoryAllocations,
) -> Result<MemoryAllocationsResponse, InternalError> {
    let ic_memory::MemoryAllocations { summary, memories } = report;
    let current_generation = summary
        .current_generation
        .ok_or_else(|| InternalError::public(crate::diagnostics::codes::STATE_INVALID))?;
    Ok(MemoryAllocationsResponse {
        current_generation,
        manager_layout_version: ic_memory::MemoryAllocationSummary::MANAGER_LAYOUT_VERSION,
        bucket_size_pages: summary.bucket_size_pages,
        bucket_size_bytes: summary.bucket_size_bytes(),
        bucket_capacity: ic_memory::MemoryAllocationSummary::BUCKET_CAPACITY,
        allocated_buckets: summary.allocated_buckets,
        remaining_buckets: summary.remaining_buckets(),
        maximum_bucket_bytes: summary.maximum_bucket_bytes(),
        physical_extent: memory_allocation_size_response(summary.physical_extent),
        virtual_extent: memory_allocation_size_response(summary.virtual_extent),
        manager_metadata_bytes: ic_memory::MemoryAllocationSummary::MANAGER_METADATA_BYTES,
        manager_header_bytes: ic_memory::MemoryAllocationSummary::MANAGER_HEADER_BYTES,
        manager_bucket_table_bytes: ic_memory::MemoryAllocationSummary::MANAGER_BUCKET_TABLE_BYTES,
        manager_padding_bytes: ic_memory::MemoryAllocationSummary::MANAGER_PADDING_BYTES,
        allocated_bucket_bytes: summary.allocated_bucket_bytes(),
        bucket_slack_bytes: summary.bucket_slack_bytes(),
        known_binding_bytes: summary.known_binding_bytes(),
        unknown_binding_bytes: summary.unknown_binding_bytes(),
        unmanaged_bytes: summary.unmanaged_bytes(),
        metadata_bytes_read: ic_memory::MemoryAllocationSummary::METADATA_BYTES_READ,
        memories: memories
            .into_iter()
            .map(|entry| memory_allocation_entry_response(entry, summary.bucket_size_pages))
            .collect(),
    })
}

fn memory_allocation_entry_response(
    entry: ic_memory::MemoryAllocation,
    bucket_size_pages: u16,
) -> MemoryAllocationEntry {
    let allocated_bytes = entry.allocated_bytes(bucket_size_pages);
    let bucket_slack_bytes = entry.bucket_slack_bytes(bucket_size_pages);
    let binding = match entry.binding {
        ic_memory::AllocationBinding::Current { stable_key, owner } => {
            MemoryAllocationBinding::Current { stable_key, owner }
        }
        ic_memory::AllocationBinding::Ledger => MemoryAllocationBinding::Ledger {
            stable_key: ic_memory::IC_MEMORY_LEDGER_STABLE_KEY.to_owned(),
            owner: ic_memory::IC_MEMORY_AUTHORITY_OWNER.to_owned(),
        },
        ic_memory::AllocationBinding::Unknown => MemoryAllocationBinding::Unknown,
    };
    MemoryAllocationEntry {
        memory_manager_id: entry.memory_manager_id,
        binding,
        pool_eligible: entry.pool_eligible,
        virtual_extent: memory_allocation_size_response(entry.virtual_extent),
        allocated_buckets: entry.allocated_buckets,
        allocated_bytes,
        bucket_slack_bytes,
        payload_bytes: None,
    }
}

fn commit_recovery_response(
    diagnostic: Option<CommitSnapshotDiagnostic>,
) -> MemoryCommitRecoveryResponse {
    let diagnostic = diagnostic.unwrap_or(CommitSnapshotDiagnostic::Empty);
    let (authoritative_generation, recovery_error) = match diagnostic {
        CommitSnapshotDiagnostic::Valid { generation } => (Some(generation), None),
        CommitSnapshotDiagnostic::Empty => (
            None,
            Some(MemoryCommitRecoveryErrorResponse::NoValidGeneration),
        ),
        CommitSnapshotDiagnostic::Invalid { .. } => (
            None,
            Some(MemoryCommitRecoveryErrorResponse::InvalidSnapshot),
        ),
    };
    MemoryCommitRecoveryResponse {
        current: commit_snapshot_response(diagnostic),
        authoritative_generation,
        recovery_error,
    }
}

fn memory_allocation_record_response(record: DiagnosticRecord) -> MemoryAllocationRecordEntry {
    let memory_size = record.memory_size.map(memory_allocation_size_response);
    let allocation = record.allocation;
    let allocation_state = allocation.state();
    MemoryAllocationRecordEntry {
        memory_manager_id: Some(allocation.slot().id()),
        stable_key: allocation.stable_key().as_str().to_string(),
        state: memory_allocation_state_response(allocation_state),
        memory_size,
        schema_version: allocation.schema().schema_version(),
    }
}

fn memory_ledger_memory_entries(
    records: &[MemoryAllocationRecordEntry],
) -> Vec<MemoryLedgerMemoryEntry> {
    records
        .iter()
        .filter_map(memory_ledger_memory_entry_response)
        .collect()
}

fn memory_ledger_memory_entry_response(
    record: &MemoryAllocationRecordEntry,
) -> Option<MemoryLedgerMemoryEntry> {
    Some(MemoryLedgerMemoryEntry {
        memory_manager_id: record.memory_manager_id?,
        stable_key: record.stable_key.clone(),
        state: record.state,
        size: record.memory_size?,
    })
}

fn memory_allocation_pool_response(
    pool: &ic_memory::MemoryAllocationPool,
) -> MemoryAllocationPoolEntry {
    MemoryAllocationPoolEntry {
        authorities: pool
            .authorities()
            .iter()
            .map(|grant| MemoryAuthorityEntry {
                owner: grant.authority().to_string(),
                key_prefix: grant.key_prefix().to_string(),
            })
            .collect(),
        excluded_ranges: pool
            .excluded_ranges()
            .iter()
            .map(|range| MemoryExcludedRangeEntry {
                start: range.start(),
                end: range.end(),
            })
            .collect(),
    }
}

const fn memory_allocation_size_response(size: DiagnosticMemorySize) -> MemoryAllocationSizeEntry {
    MemoryAllocationSizeEntry {
        wasm_pages: size.wasm_pages,
        bytes: size.bytes(),
    }
}

const fn memory_allocation_state_response(state: AllocationState) -> MemoryAllocationState {
    match state {
        AllocationState::Reserved => MemoryAllocationState::Reserved,
        AllocationState::Active => MemoryAllocationState::Active,
        AllocationState::Retired => MemoryAllocationState::Retired,
    }
}

const fn commit_snapshot_response(slot: CommitSnapshotDiagnostic) -> MemoryCommitSnapshotResponse {
    match slot {
        CommitSnapshotDiagnostic::Empty => MemoryCommitSnapshotResponse {
            present: false,
            generation: None,
            valid: false,
        },
        CommitSnapshotDiagnostic::Valid { generation } => MemoryCommitSnapshotResponse {
            present: true,
            generation: Some(generation),
            valid: true,
        },
        CommitSnapshotDiagnostic::Invalid { generation } => MemoryCommitSnapshotResponse {
            present: true,
            generation: Some(generation),
            valid: false,
        },
    }
}
