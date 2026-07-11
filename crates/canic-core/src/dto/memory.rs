use crate::dto::prelude::*;

pub use crate::domain::memory::{
    MemoryAllocationState, MemoryCommitRecoveryErrorResponse, MemoryRangeAuthorityMode,
};

///
/// MemoryLedgerResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryLedgerResponse {
    pub ledger_schema_version: u32,
    pub physical_format_id: u32,
    pub current_generation: u64,
    pub commit_recovery: MemoryCommitRecoveryResponse,
    pub authorities: Vec<MemoryRangeAuthorityEntry>,
    pub memories: Vec<MemoryLedgerMemoryEntry>,
    pub records: Vec<MemoryAllocationRecordEntry>,
    pub generations: Vec<MemoryLedgerGenerationEntry>,
}

///
/// MemoryCommitRecoveryResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryCommitRecoveryResponse {
    pub slot0: MemoryCommitSlotResponse,
    pub slot1: MemoryCommitSlotResponse,
    pub authoritative_generation: Option<u64>,
    pub recovery_error: Option<MemoryCommitRecoveryErrorResponse>,
}

///
/// MemoryCommitSlotResponse
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryCommitSlotResponse {
    pub present: bool,
    pub generation: Option<u64>,
    pub valid: bool,
}

///
/// MemoryRangeAuthorityEntry
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryRangeAuthorityEntry {
    pub owner: String,
    pub start: u8,
    pub end: u8,
    pub mode: MemoryRangeAuthorityMode,
    pub purpose: String,
}

///
/// MemoryLedgerMemoryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryLedgerMemoryEntry {
    pub memory_manager_id: u8,
    pub stable_key: String,
    pub state: MemoryAllocationState,
    pub size: MemoryAllocationSizeEntry,
}

///
/// MemoryAllocationRecordEntry
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryAllocationRecordEntry {
    pub memory_manager_id: Option<u8>,
    pub stable_key: String,
    pub state: MemoryAllocationState,
    pub memory_size: Option<MemoryAllocationSizeEntry>,
    pub first_generation: u64,
    pub last_seen_generation: u64,
    pub retired_generation: Option<u64>,
    pub schema_history: Vec<MemorySchemaMetadataEntry>,
}

///
/// MemoryAllocationSizeEntry
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryAllocationSizeEntry {
    pub wasm_pages: u64,
    pub bytes: u64,
}

///
/// MemorySchemaMetadataEntry
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemorySchemaMetadataEntry {
    pub generation: u64,
    pub schema_version: Option<u32>,
    pub schema_fingerprint: Option<String>,
}

///
/// MemoryLedgerGenerationEntry
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryLedgerGenerationEntry {
    pub generation: u64,
    pub parent_generation: Option<u64>,
    pub runtime_fingerprint: Option<String>,
    pub declaration_count: u32,
    pub committed_at: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[test]
    fn memory_enums_roundtrip_candid_with_existing_variant_labels() {
        assert_enum_candid_contract(MemoryCommitRecoveryErrorResponse::InvalidCommitSlots);
        assert_enum_candid_contract(MemoryCommitRecoveryErrorResponse::UnexpectedGeneration);
        assert_enum_candid_contract(MemoryRangeAuthorityMode::Allowed);
        assert_enum_candid_contract(MemoryAllocationState::Retired);
    }

    fn assert_enum_candid_contract<T>(value: T)
    where
        T: CandidType + Clone + Debug + DeserializeOwned + Eq,
    {
        let bytes = Encode!(&value).expect("encode memory enum");
        let decoded = Decode!(&bytes, T).expect("decode memory enum");

        assert_eq!(decoded, value);
    }
}
