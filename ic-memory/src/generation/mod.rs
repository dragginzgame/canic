use crate::{declaration::DeclarationSnapshot, ledger::GenerationRecord};
use serde::{Deserialize, Serialize};

///
/// GenerationMutation
///
/// One staged allocation-history mutation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GenerationMutation {
    /// Add or confirm an active allocation declaration.
    Declare,
    /// Add or confirm a reserved allocation declaration.
    Reserve,
    /// Explicitly retire an allocation.
    Retire,
    /// Record schema metadata drift for diagnostics.
    RecordSchemaMetadata,
}

///
/// StagedGeneration
///
/// Generation prepared for atomic commit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StagedGeneration {
    /// Generation number to commit.
    pub generation: u64,
    /// Parent generation that remains authoritative until commit succeeds.
    pub parent_generation: u64,
    /// Snapshot being committed.
    pub snapshot: DeclarationSnapshot,
    /// Staged mutations included in this generation.
    pub mutations: Vec<GenerationMutation>,
}

///
/// GenerationCommit
///
/// Protected commit metadata for a generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GenerationCommit {
    /// Committed generation record.
    pub record: GenerationRecord,
    /// Protected commit checksum supplied by the physical format.
    pub checksum: u64,
    /// Physical commit marker supplied by the physical format.
    pub commit_marker: u64,
}
