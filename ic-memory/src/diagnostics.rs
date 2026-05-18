use crate::{
    ledger::{AllocationLedger, AllocationRecord, GenerationRecord},
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

impl DiagnosticExport {
    /// Build a read-only diagnostic export from an allocation ledger.
    #[must_use]
    pub fn from_ledger(ledger: &AllocationLedger, ledger_anchor: AllocationSlotDescriptor) -> Self {
        Self {
            ledger_schema_version: ledger.ledger_schema_version,
            physical_format_id: ledger.physical_format_id,
            current_generation: ledger.current_generation,
            ledger_anchor,
            records: ledger
                .allocation_history
                .records
                .iter()
                .cloned()
                .map(|allocation| DiagnosticRecord { allocation })
                .collect(),
            generations: ledger
                .allocation_history
                .generations
                .iter()
                .cloned()
                .map(|generation| DiagnosticGeneration { generation })
                .collect(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        declaration::AllocationDeclaration,
        ledger::{AllocationHistory, AllocationRecord, AllocationState},
        schema::SchemaMetadata,
    };

    #[test]
    fn diagnostic_export_copies_ledger_records() {
        let declaration = AllocationDeclaration::new(
            "app.users.v1",
            AllocationSlotDescriptor::memory_manager(100),
            None,
            SchemaMetadata::default(),
        )
        .expect("declaration");
        let ledger = AllocationLedger {
            ledger_schema_version: 1,
            physical_format_id: 1,
            current_generation: 3,
            allocation_history: AllocationHistory {
                records: vec![AllocationRecord::from_declaration(
                    3,
                    declaration,
                    AllocationState::Active,
                )],
                generations: vec![GenerationRecord {
                    generation: 3,
                    parent_generation: Some(2),
                    runtime_fingerprint: Some("wasm:abc123".to_string()),
                    declaration_count: 1,
                    committed_at: None,
                }],
            },
        };

        let export =
            DiagnosticExport::from_ledger(&ledger, AllocationSlotDescriptor::memory_manager(0));

        assert_eq!(export.current_generation, 3);
        assert_eq!(export.records.len(), 1);
        assert_eq!(export.generations.len(), 1);
        assert_eq!(
            export.ledger_anchor,
            AllocationSlotDescriptor::memory_manager(0)
        );
    }
}
