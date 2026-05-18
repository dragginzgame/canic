use crate::{
    ledger::{AllocationRecord, GenerationRecord},
    slot::AllocationSlotDescriptor,
};
use serde::{Deserialize, Serialize};

///
/// DiagnosticExport
///
/// Read-only machine-readable allocation ledger export.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticExport {
    /// Ledger schema version.
    pub ledger_schema_version: u32,
    /// Physical format identifier.
    pub physical_format_id: u32,
    /// Current committed generation.
    pub current_generation: u64,
    /// Ledger anchor descriptor.
    pub ledger_anchor: AllocationSlotDescriptor,
    /// Allocation records.
    pub records: Vec<DiagnosticRecord>,
    /// Generation records.
    pub generations: Vec<DiagnosticGeneration>,
}

///
/// DiagnosticRecord
///
/// Read-only diagnostic allocation record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticRecord {
    /// Allocation record.
    pub allocation: AllocationRecord,
}

///
/// DiagnosticGeneration
///
/// Read-only diagnostic generation record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticGeneration {
    /// Generation record.
    pub generation: GenerationRecord,
}
