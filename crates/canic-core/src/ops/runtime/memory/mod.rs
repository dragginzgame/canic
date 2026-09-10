//! Module: ops::runtime::memory
//!
//! Responsibility: bootstrap memory registry TLS and expose memory diagnostics.
//! Does not own: memory schema declarations, stable records, or DTO schema.
//! Boundary: maps memory runtime diagnostics into ops query responses.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    domain::memory::{
        MemoryAllocationBinding, MemoryAllocationState, MemoryCommitRecoveryErrorResponse,
        MemoryRangeAuthorityMode,
    },
    dto::memory::{
        MemoryAllocationEntry, MemoryAllocationRangeClaim, MemoryAllocationRecordEntry,
        MemoryAllocationSizeEntry, MemoryAllocationsResponse, MemoryCommitRecoveryResponse,
        MemoryCommitSlotResponse, MemoryLedgerGenerationEntry, MemoryLedgerMemoryEntry,
        MemoryLedgerResponse, MemoryRangeAuthorityEntry, MemorySchemaMetadataEntry,
    },
    memory::{self, ledger, registry::MemoryRegistryError, runtime::init_eager_tls},
};
use ic_memory::{
    AllocationState, CommitRecoveryError, CommitSlotDiagnostic, CommitStoreDiagnostic,
    DiagnosticGeneration, DiagnosticMemorySize, DiagnosticMemorySizeOutcome, DiagnosticRecord,
    MemoryManagerRangeMode, SchemaMetadataRecord,
};
use thiserror::Error as ThisError;

///
/// MemoryRegistryOpsError
///
/// Typed failure surface for memory registry bootstrap and diagnostics.
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryOpsError {
    // this error comes from the Canic memory runtime boundary
    #[error(transparent)]
    Registry(#[from] MemoryRegistryError),
    // this error comes from the generic ic-memory runtime boundary
    #[error(transparent)]
    Runtime(#[from] ic_memory::RuntimeBootstrapError<MemoryRegistryError>),
    // this error comes from the generic ic-memory runtime diagnostic boundary
    #[error(transparent)]
    Diagnostic(#[from] ic_memory::RuntimeDiagnosticError),
    // this error comes from entering the default ic-memory TLS runtime
    #[error(transparent)]
    State(#[from] ic_memory::RuntimeStateError),
}

impl From<MemoryRegistryOpsError> for InternalError {
    fn from(err: MemoryRegistryOpsError) -> Self {
        let code = match err {
            MemoryRegistryOpsError::Registry(_) | MemoryRegistryOpsError::Runtime(_) => {
                crate::diagnostics::codes::STORAGE_INVALID_STATE
            }
            MemoryRegistryOpsError::Diagnostic(_) | MemoryRegistryOpsError::State(_) => {
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
        let report = ic_memory::default_memory_manager_memory_allocations()
            .map_err(MemoryRegistryOpsError::from)?;
        memory_allocations_response(report)
    }

    // Run eager TLS touches after the registry validates stable-memory slots.
    pub fn init_eager_tls() {
        init_eager_tls();
    }

    // Initialize the stable-memory registry for this crate and summarize the layout.
    pub(crate) fn init_registry() -> Result<(), InternalError> {
        memory::bootstrap_default_memory_manager().map_err(MemoryRegistryOpsError::from)?;
        Ok(())
    }

    // Run the full synchronous Canic memory bootstrap and return the committed layout.
    pub fn bootstrap_registry() -> Result<(), InternalError> {
        Self::init_registry()?;
        Self::init_eager_tls();
        Ok(())
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

        let authorities = snapshot
            .authorities
            .into_iter()
            .map(memory_range_authority_entry_response)
            .collect();

        let records: Vec<MemoryAllocationRecordEntry> = snapshot
            .export
            .records
            .into_iter()
            .map(memory_allocation_record_response)
            .collect();
        let memories = memory_ledger_memory_entries(&records);
        let generations = snapshot
            .export
            .generations
            .into_iter()
            .map(memory_ledger_generation_response)
            .collect();

        Ok(MemoryLedgerResponse {
            ledger_schema_version: crate::memory::ledger::MEMORY_LEDGER_SCHEMA_VERSION,
            physical_format_id: crate::memory::ledger::MEMORY_PHYSICAL_FORMAT_ID,
            current_generation: snapshot.export.current_generation,
            commit_recovery: commit_recovery_response(snapshot.export.commit_recovery),
            authorities,
            memories,
            records,
            generations,
        })
    }
}

fn memory_allocations_response(
    report: ic_memory::MemoryAllocations,
) -> Result<MemoryAllocationsResponse, InternalError> {
    let current_generation = report
        .current_generation
        .ok_or_else(|| InternalError::public(crate::diagnostics::codes::STATE_INVALID))?;
    Ok(MemoryAllocationsResponse {
        current_generation,
        manager_layout_version: report.manager_layout_version,
        bucket_size_pages: report.bucket_size_pages,
        bucket_size_bytes: report.bucket_size_bytes,
        bucket_capacity: report.bucket_capacity,
        allocated_buckets: report.allocated_buckets,
        remaining_buckets: report.remaining_buckets,
        maximum_bucket_bytes: report.maximum_bucket_bytes,
        physical_extent: memory_allocation_size_response(report.physical_extent),
        virtual_extent: memory_allocation_size_response(report.virtual_extent),
        manager_metadata_bytes: report.manager_metadata_bytes,
        manager_header_bytes: report.manager_header_bytes,
        manager_bucket_table_bytes: report.manager_bucket_table_bytes,
        manager_padding_bytes: report.manager_padding_bytes,
        allocated_bucket_bytes: report.allocated_bucket_bytes,
        bucket_slack_bytes: report.bucket_slack_bytes,
        known_binding_bytes: report.known_binding_bytes,
        unknown_binding_bytes: report.unknown_binding_bytes,
        unmanaged_bytes: report.unmanaged_bytes,
        metadata_bytes_read: report.metadata_bytes_read,
        memories: report
            .memories
            .into_iter()
            .map(memory_allocation_entry_response)
            .collect(),
    })
}

fn memory_allocation_entry_response(entry: ic_memory::MemoryAllocation) -> MemoryAllocationEntry {
    let binding = match entry.binding {
        ic_memory::AllocationBinding::Current { stable_key, owner } => {
            MemoryAllocationBinding::Current { stable_key, owner }
        }
        ic_memory::AllocationBinding::Ledger { stable_key, owner } => {
            MemoryAllocationBinding::Ledger { stable_key, owner }
        }
        ic_memory::AllocationBinding::Unknown => MemoryAllocationBinding::Unknown,
    };
    MemoryAllocationEntry {
        memory_manager_id: entry.memory_manager_id,
        binding,
        range_claim: entry.range_claim.map(|claim| MemoryAllocationRangeClaim {
            authority: claim.authority,
            mode: memory_range_authority_mode(claim.mode),
        }),
        virtual_extent: memory_allocation_size_response(entry.virtual_extent),
        allocated_buckets: entry.allocated_buckets,
        allocated_bytes: entry.allocated_bytes,
        bucket_slack_bytes: entry.bucket_slack_bytes,
        payload_bytes: entry.payload_bytes,
    }
}

const fn memory_range_authority_mode(mode: MemoryManagerRangeMode) -> MemoryRangeAuthorityMode {
    match mode {
        MemoryManagerRangeMode::Reserved => MemoryRangeAuthorityMode::Reserved,
        MemoryManagerRangeMode::Allowed => MemoryRangeAuthorityMode::Allowed,
    }
}

fn commit_recovery_response(
    diagnostic: Option<CommitStoreDiagnostic>,
) -> MemoryCommitRecoveryResponse {
    let diagnostic = diagnostic.unwrap_or(CommitStoreDiagnostic {
        slot0: CommitSlotDiagnostic::Empty,
        slot1: CommitSlotDiagnostic::Empty,
        recovery: Err(CommitRecoveryError::NoValidGeneration),
    });
    let (authoritative_generation, recovery_error) = match diagnostic.recovery {
        Ok(generation) => (Some(generation), None),
        Err(error) => (None, Some(commit_recovery_error_response(error))),
    };
    MemoryCommitRecoveryResponse {
        slot0: commit_slot_response(diagnostic.slot0),
        slot1: commit_slot_response(diagnostic.slot1),
        authoritative_generation,
        recovery_error,
    }
}

fn memory_allocation_record_response(record: DiagnosticRecord) -> MemoryAllocationRecordEntry {
    let memory_size = record
        .memory_size
        .and_then(memory_allocation_size_outcome_response);
    let allocation = record.allocation;
    let allocation_state = allocation.state();
    MemoryAllocationRecordEntry {
        memory_manager_id: allocation.slot().memory_manager_id().ok(),
        stable_key: allocation.stable_key().as_str().to_string(),
        state: memory_allocation_state_response(allocation_state),
        memory_size,
        first_generation: allocation.first_generation(),
        last_seen_generation: allocation.last_seen_generation(),
        retired_generation: allocation_retired_generation(allocation_state),
        schema_history: allocation
            .schema_history()
            .iter()
            .map(memory_schema_metadata_response)
            .collect(),
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

fn memory_range_authority_entry_response(
    authority: ic_memory::MemoryManagerAuthorityRecord,
) -> MemoryRangeAuthorityEntry {
    let range = authority.range();
    MemoryRangeAuthorityEntry {
        owner: authority.authority().to_string(),
        start: range.start(),
        end: range.end(),
        mode: memory_range_authority_mode(authority.mode()),
        purpose: authority.purpose().unwrap_or_default().to_string(),
    }
}

const fn memory_allocation_size_response(size: DiagnosticMemorySize) -> MemoryAllocationSizeEntry {
    MemoryAllocationSizeEntry {
        wasm_pages: size.wasm_pages,
        bytes: size.bytes,
    }
}

fn memory_allocation_size_outcome_response(
    outcome: DiagnosticMemorySizeOutcome,
) -> Option<MemoryAllocationSizeEntry> {
    match outcome {
        DiagnosticMemorySizeOutcome::Measured(size) => Some(memory_allocation_size_response(size)),
        DiagnosticMemorySizeOutcome::Failed(_) => None,
    }
}

const fn memory_allocation_state_response(state: AllocationState) -> MemoryAllocationState {
    match state {
        AllocationState::Reserved => MemoryAllocationState::Reserved,
        AllocationState::Active => MemoryAllocationState::Active,
        AllocationState::Retired { .. } => MemoryAllocationState::Retired,
    }
}

const fn allocation_retired_generation(state: AllocationState) -> Option<u64> {
    match state {
        AllocationState::Retired { generation } => Some(generation),
        AllocationState::Reserved | AllocationState::Active => None,
    }
}

const fn memory_schema_metadata_response(
    record: &SchemaMetadataRecord,
) -> MemorySchemaMetadataEntry {
    MemorySchemaMetadataEntry {
        generation: record.generation(),
        schema_version: record.schema().schema_version(),
        schema_fingerprint: None,
    }
}

fn memory_ledger_generation_response(
    generation: DiagnosticGeneration,
) -> MemoryLedgerGenerationEntry {
    let generation = generation.generation;
    MemoryLedgerGenerationEntry {
        generation: generation.generation(),
        parent_generation: Some(generation.parent_generation()),
        runtime_fingerprint: generation.runtime_fingerprint().map(str::to_string),
        declaration_count: generation.declaration_count(),
        committed_at: generation.committed_at(),
    }
}

const fn commit_slot_response(slot: CommitSlotDiagnostic) -> MemoryCommitSlotResponse {
    match slot {
        CommitSlotDiagnostic::Empty => MemoryCommitSlotResponse {
            present: false,
            generation: None,
            valid: false,
        },
        CommitSlotDiagnostic::Valid { generation } => MemoryCommitSlotResponse {
            present: true,
            generation: Some(generation),
            valid: true,
        },
        CommitSlotDiagnostic::Invalid { generation } => MemoryCommitSlotResponse {
            present: true,
            generation: Some(generation),
            valid: false,
        },
    }
}

const fn commit_recovery_error_response(
    err: CommitRecoveryError,
) -> MemoryCommitRecoveryErrorResponse {
    match err {
        CommitRecoveryError::NoValidGeneration => {
            MemoryCommitRecoveryErrorResponse::NoValidGeneration
        }
        CommitRecoveryError::InvalidCommitSlots { .. } => {
            MemoryCommitRecoveryErrorResponse::InvalidCommitSlots
        }
        CommitRecoveryError::AmbiguousGeneration { .. } => {
            MemoryCommitRecoveryErrorResponse::AmbiguousGeneration
        }
        CommitRecoveryError::GenerationOverflow { .. } => {
            MemoryCommitRecoveryErrorResponse::GenerationOverflow
        }
        CommitRecoveryError::UnexpectedGeneration { .. } => {
            MemoryCommitRecoveryErrorResponse::UnexpectedGeneration
        }
        _ => MemoryCommitRecoveryErrorResponse::Unknown,
    }
}
