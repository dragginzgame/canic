use crate::{
    declaration::AllocationDeclaration, key::StableKey, schema::SchemaMetadata,
    session::ValidatedAllocations, slot::AllocationSlotDescriptor,
};
use serde::{Deserialize, Serialize};

///
/// AllocationLedger
///
/// Durable root of allocation history.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllocationLedger {
    /// Ledger schema version.
    pub ledger_schema_version: u32,
    /// Physical encoding format identifier.
    pub physical_format_id: u32,
    /// Current committed generation selected by recovery.
    pub current_generation: u64,
    /// Historical allocation facts.
    pub allocation_history: AllocationHistory,
}

///
/// AllocationHistory
///
/// Durable allocation records and generation history.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllocationHistory {
    /// Stable-key allocation records.
    pub records: Vec<AllocationRecord>,
    /// Committed generation records.
    pub generations: Vec<GenerationRecord>,
}

///
/// AllocationRecord
///
/// Durable ownership record for one stable key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllocationRecord {
    /// Stable key that owns the slot.
    pub stable_key: StableKey,
    /// Durable allocation slot owned by the key.
    pub slot: AllocationSlotDescriptor,
    /// Current allocation lifecycle state.
    pub state: AllocationState,
    /// First committed generation that recorded this allocation.
    pub first_generation: u64,
    /// Latest committed generation that observed this allocation declaration.
    pub last_seen_generation: u64,
    /// Generation that explicitly retired this allocation.
    pub retired_generation: Option<u64>,
    /// Per-generation schema metadata history.
    pub schema_history: Vec<SchemaMetadataRecord>,
}

///
/// AllocationState
///
/// Allocation lifecycle state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AllocationState {
    /// Slot is reserved for a future allocation identity.
    Reserved,
    /// Slot is active and may be opened after validation.
    Active,
    /// Slot was explicitly retired and remains tombstoned.
    Retired,
}

///
/// SchemaMetadataRecord
///
/// Schema metadata observed in one committed generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchemaMetadataRecord {
    /// Generation that declared this schema metadata.
    pub generation: u64,
    /// Schema metadata declared by that generation.
    pub schema: SchemaMetadata,
}

///
/// GenerationRecord
///
/// Diagnostic metadata for one committed ledger generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GenerationRecord {
    /// Committed generation number.
    pub generation: u64,
    /// Parent generation, if recorded.
    pub parent_generation: Option<u64>,
    /// Optional binary/runtime fingerprint.
    pub runtime_fingerprint: Option<String>,
    /// Number of declarations in the generation.
    pub declaration_count: u32,
    /// Optional commit timestamp supplied by the integration layer.
    pub committed_at: Option<u64>,
}

impl AllocationRecord {
    /// Create a new allocation record from a declaration.
    #[must_use]
    pub fn from_declaration(
        generation: u64,
        declaration: AllocationDeclaration,
        state: AllocationState,
    ) -> Self {
        Self {
            stable_key: declaration.stable_key,
            slot: declaration.slot,
            state,
            first_generation: generation,
            last_seen_generation: generation,
            retired_generation: None,
            schema_history: vec![SchemaMetadataRecord {
                generation,
                schema: declaration.schema,
            }],
        }
    }

    fn observe_declaration(&mut self, generation: u64, declaration: &AllocationDeclaration) {
        self.last_seen_generation = generation;

        let latest_schema = self.schema_history.last().map(|record| &record.schema);
        if latest_schema != Some(&declaration.schema) {
            self.schema_history.push(SchemaMetadataRecord {
                generation,
                schema: declaration.schema.clone(),
            });
        }
    }
}

impl AllocationLedger {
    /// Return a copy of the ledger with `validated` recorded as the next generation.
    ///
    /// This is a pure logical update. Physical atomicity is the responsibility of
    /// the substrate commit protocol.
    #[must_use]
    pub fn stage_validated_generation(
        &self,
        validated: &ValidatedAllocations,
        committed_at: Option<u64>,
    ) -> Self {
        let next_generation = self.current_generation.saturating_add(1);
        let mut next = self.clone();
        next.current_generation = next_generation;
        let declaration_count = u32::try_from(validated.declarations.len()).unwrap_or(u32::MAX);

        for declaration in &validated.declarations {
            record_declaration(&mut next, next_generation, declaration);
        }

        next.allocation_history.generations.push(GenerationRecord {
            generation: next_generation,
            parent_generation: Some(self.current_generation),
            runtime_fingerprint: None,
            declaration_count,
            committed_at,
        });

        next
    }
}

fn record_declaration(
    ledger: &mut AllocationLedger,
    generation: u64,
    declaration: &AllocationDeclaration,
) {
    if let Some(record) = ledger
        .allocation_history
        .records
        .iter_mut()
        .find(|record| record.stable_key == declaration.stable_key)
    {
        record.observe_declaration(generation, declaration);
        return;
    }

    ledger
        .allocation_history
        .records
        .push(AllocationRecord::from_declaration(
            generation,
            declaration.clone(),
            AllocationState::Active,
        ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        declaration::DeclarationSnapshot, schema::SchemaMetadata, session::ValidatedAllocations,
    };

    fn declaration(key: &str, id: u8, schema_version: Option<u32>) -> AllocationDeclaration {
        AllocationDeclaration::new(
            key,
            AllocationSlotDescriptor::memory_manager(id),
            None,
            SchemaMetadata {
                schema_version,
                schema_fingerprint: None,
            },
        )
        .expect("declaration")
    }

    fn ledger() -> AllocationLedger {
        AllocationLedger {
            ledger_schema_version: 1,
            physical_format_id: 1,
            current_generation: 3,
            allocation_history: AllocationHistory::default(),
        }
    }

    #[test]
    fn stage_validated_generation_records_new_allocations() {
        let declarations = vec![declaration("app.users.v1", 100, Some(1))];
        let validated = ValidatedAllocations {
            generation: 3,
            declarations,
        };

        let staged = ledger().stage_validated_generation(&validated, Some(42));

        assert_eq!(staged.current_generation, 4);
        assert_eq!(staged.allocation_history.records.len(), 1);
        assert_eq!(staged.allocation_history.records[0].first_generation, 4);
        assert_eq!(staged.allocation_history.generations[0].generation, 4);
        assert_eq!(
            staged.allocation_history.generations[0].committed_at,
            Some(42)
        );
    }

    #[test]
    fn stage_validated_generation_preserves_omitted_records() {
        let first = ValidatedAllocations {
            generation: 3,
            declarations: vec![
                declaration("app.users.v1", 100, Some(1)),
                declaration("app.orders.v1", 101, Some(1)),
            ],
        };
        let second = ValidatedAllocations {
            generation: 4,
            declarations: vec![declaration("app.users.v1", 100, Some(1))],
        };

        let staged = ledger().stage_validated_generation(&first, None);
        let staged = staged.stage_validated_generation(&second, None);

        assert_eq!(staged.current_generation, 5);
        assert_eq!(staged.allocation_history.records.len(), 2);
        let omitted = staged
            .allocation_history
            .records
            .iter()
            .find(|record| record.stable_key.as_str() == "app.orders.v1")
            .expect("omitted record");
        assert_eq!(omitted.state, AllocationState::Active);
        assert_eq!(omitted.last_seen_generation, 4);
    }

    #[test]
    fn stage_validated_generation_records_schema_metadata_history() {
        let first = ValidatedAllocations {
            generation: 3,
            declarations: vec![declaration("app.users.v1", 100, Some(1))],
        };
        let second = ValidatedAllocations {
            generation: 4,
            declarations: vec![declaration("app.users.v1", 100, Some(2))],
        };

        let staged = ledger().stage_validated_generation(&first, None);
        let staged = staged.stage_validated_generation(&second, None);
        let record = &staged.allocation_history.records[0];

        assert_eq!(record.schema_history.len(), 2);
        assert_eq!(record.schema_history[0].generation, 4);
        assert_eq!(record.schema_history[1].generation, 5);
    }

    #[test]
    fn snapshot_can_feed_validated_generation() {
        let snapshot = DeclarationSnapshot::new(vec![declaration("app.users.v1", 100, None)])
            .expect("snapshot");
        let validated = ValidatedAllocations {
            generation: 3,
            declarations: snapshot.declarations,
        };

        let staged = ledger().stage_validated_generation(&validated, None);

        assert_eq!(staged.allocation_history.records.len(), 1);
    }
}
