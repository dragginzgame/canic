//! Module: ops::runtime::memory
//!
//! Responsibility: bootstrap memory registry TLS and expose memory diagnostics.
//! Does not own: memory schema declarations, stable records, or DTO schema.
//! Boundary: maps memory runtime diagnostics into ops query responses.

use crate::{
    InternalError,
    dto::memory::{
        MemoryAllocationRecordEntry, MemoryAllocationSizeEntry, MemoryAllocationState,
        MemoryCommitRecoveryErrorResponse, MemoryCommitRecoveryResponse, MemoryCommitSlotResponse,
        MemoryLedgerGenerationEntry, MemoryLedgerMemoryEntry, MemoryLedgerResponse,
        MemoryRangeAuthorityEntry, MemoryRangeAuthorityMode, MemorySchemaMetadataEntry,
    },
    memory::{self, ledger, registry::MemoryRegistryError, runtime::init_eager_tls},
    ops::runtime::RuntimeOpsError,
};
use ic_memory::{
    AllocationState, CommitRecoveryError, CommitSlotDiagnostic, CommitStoreDiagnostic,
    DiagnosticGeneration, DiagnosticMemorySize, DiagnosticRecord, MemoryManagerRangeMode,
    SchemaMetadataRecord,
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
}

impl From<MemoryRegistryOpsError> for InternalError {
    fn from(err: MemoryRegistryOpsError) -> Self {
        RuntimeOpsError::MemoryRegistryOps(err).into()
    }
}

///
/// MemoryRegistryOps
///
/// Operations-layer facade for memory registry bootstrap and diagnostics.
///

pub struct MemoryRegistryOps;

impl MemoryRegistryOps {
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

    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub fn is_initialized() -> bool {
        ic_memory::runtime::is_default_memory_manager_bootstrapped()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn ensure_bootstrap() -> Result<(), InternalError> {
        if Self::is_initialized() {
            return Ok(());
        }

        Self::bootstrap_registry()?;
        Ok(())
    }

    // Read the committed ABI ledger using the restricted diagnostic path.
    pub fn ledger_snapshot() -> Result<MemoryLedgerResponse, InternalError> {
        #[cfg(target_arch = "wasm32")]
        let snapshot = ledger::try_diagnostic_snapshot().map_err(MemoryRegistryOpsError::from)?;

        #[cfg(not(target_arch = "wasm32"))]
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
        slot0: CommitSlotDiagnostic {
            present: false,
            generation: None,
            valid: false,
        },
        slot1: CommitSlotDiagnostic {
            present: false,
            generation: None,
            valid: false,
        },
        authoritative_generation: None,
        recovery_error: Some(CommitRecoveryError::NoValidGeneration),
    });
    MemoryCommitRecoveryResponse {
        slot0: commit_slot_response(diagnostic.slot0),
        slot1: commit_slot_response(diagnostic.slot1),
        authoritative_generation: diagnostic.authoritative_generation,
        recovery_error: diagnostic
            .recovery_error
            .map(commit_recovery_error_response),
    }
}

fn memory_allocation_record_response(record: DiagnosticRecord) -> MemoryAllocationRecordEntry {
    let memory_size = record.memory_size.map(memory_allocation_size_response);
    let allocation = record.allocation;
    MemoryAllocationRecordEntry {
        memory_manager_id: allocation.slot().memory_manager_id().ok(),
        stable_key: allocation.stable_key().as_str().to_string(),
        state: memory_allocation_state_response(allocation.state()),
        memory_size,
        first_generation: allocation.first_generation(),
        last_seen_generation: allocation.last_seen_generation(),
        retired_generation: allocation.retired_generation(),
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

const fn memory_allocation_state_response(state: AllocationState) -> MemoryAllocationState {
    match state {
        AllocationState::Reserved => MemoryAllocationState::Reserved,
        AllocationState::Active => MemoryAllocationState::Active,
        AllocationState::Retired => MemoryAllocationState::Retired,
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
    MemoryCommitSlotResponse {
        present: slot.present,
        generation: slot.generation,
        valid: slot.valid,
    }
}

const fn commit_recovery_error_response(
    err: CommitRecoveryError,
) -> MemoryCommitRecoveryErrorResponse {
    match err {
        CommitRecoveryError::NoValidGeneration => {
            MemoryCommitRecoveryErrorResponse::NoValidGeneration
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ic_memory::{
        AllocationDeclaration, AllocationHistory, AllocationLedger, AllocationSlotDescriptor,
        SchemaMetadata,
    };

    #[test]
    fn memory_allocation_record_response_includes_live_backing_memory_size() {
        let declaration = AllocationDeclaration::new(
            "app.users.v1",
            AllocationSlotDescriptor::memory_manager(100).expect("usable slot"),
            None,
            SchemaMetadata::default(),
        )
        .expect("declaration");
        let ledger = AllocationLedger::new_committed(0, AllocationHistory::default())
            .expect("genesis ledger")
            .stage_reservation_generation(&[declaration], None)
            .expect("reservation generation");
        let record = DiagnosticRecord {
            allocation: ledger.allocation_history().records()[0].clone(),
            memory_size: Some(DiagnosticMemorySize::from_wasm_pages(3)),
        };

        let response = memory_allocation_record_response(record);

        assert_eq!(
            response.memory_size,
            Some(MemoryAllocationSizeEntry {
                wasm_pages: 3,
                bytes: 196_608,
            })
        );
        assert_eq!(
            memory_ledger_memory_entry_response(&response),
            Some(MemoryLedgerMemoryEntry {
                memory_manager_id: 100,
                stable_key: "app.users.v1".to_string(),
                state: MemoryAllocationState::Reserved,
                size: MemoryAllocationSizeEntry {
                    wasm_pages: 3,
                    bytes: 196_608,
                },
            })
        );
    }
}
