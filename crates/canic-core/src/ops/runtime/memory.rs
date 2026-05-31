//! Runtime memory registry primitives.
//! Owns TLS setup for memory registry initialization.

use crate::{
    InternalError,
    dto::memory::{
        MemoryAllocationRecordEntry, MemoryAllocationState, MemoryCommitRecoveryErrorResponse,
        MemoryCommitRecoveryResponse, MemoryCommitSlotResponse, MemoryLedgerGenerationEntry,
        MemoryLedgerResponse, MemoryRangeAuthorityEntry, MemoryRangeAuthorityMode,
        MemorySchemaMetadataEntry,
    },
    memory::{self, ledger, registry::MemoryRegistryError, runtime::init_eager_tls},
    ops::runtime::RuntimeOpsError,
};
use ic_memory::{
    AllocationState, CommitRecoveryError, CommitSlotDiagnostic, CommitStoreDiagnostic,
    DiagnosticGeneration, DiagnosticRecord, MemoryManagerRangeMode, SchemaMetadataRecord,
};
use thiserror::Error as ThisError;

///
/// MemoryRegistryOpsError
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
            .map(|authority| MemoryRangeAuthorityEntry {
                owner: authority.authority,
                start: authority.range.start(),
                end: authority.range.end(),
                mode: memory_range_authority_mode(authority.mode),
                purpose: authority.purpose.unwrap_or_default(),
            })
            .collect();

        let records = snapshot
            .export
            .records
            .into_iter()
            .map(memory_allocation_record_response)
            .collect();
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
    let allocation = record.allocation;
    MemoryAllocationRecordEntry {
        memory_manager_id: allocation.slot().memory_manager_id().ok(),
        stable_key: allocation.stable_key().as_str().to_string(),
        state: memory_allocation_state_response(allocation.state()),
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
        schema_version: record.schema().schema_version,
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
    }
}
