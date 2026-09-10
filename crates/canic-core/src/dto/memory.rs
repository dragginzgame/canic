use crate::dto::prelude::*;

pub use crate::domain::memory::{
    MemoryAllocationBinding, MemoryAllocationState, MemoryCommitRecoveryErrorResponse,
    MemoryRangeAuthorityMode,
};

/// Measured physical and virtual allocations exposed by protected observations.
/// Covers all usable IDs, including unknown bindings and the substrate ledger.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryAllocationsResponse {
    pub current_generation: u64,
    pub manager_layout_version: u8,
    /// Actual persisted size, never a requested/default assumption.
    pub bucket_size_pages: u16,
    pub bucket_size_bytes: u64,
    pub bucket_capacity: u32,
    pub allocated_buckets: u16,
    pub remaining_buckets: u32,
    pub maximum_bucket_bytes: u64,
    /// Backing extent: IC stable memory in canisters, vector memory on native hosts.
    pub physical_extent: MemoryAllocationSizeEntry,
    /// Addressable capacity, not stored payload occupancy.
    pub virtual_extent: MemoryAllocationSizeEntry,
    pub manager_metadata_bytes: u64,
    pub manager_header_bytes: u64,
    pub manager_bucket_table_bytes: u64,
    pub manager_padding_bytes: u64,
    pub allocated_bucket_bytes: u64,
    pub bucket_slack_bytes: u64,
    pub known_binding_bytes: u64,
    pub unknown_binding_bytes: u64,
    /// Physical bytes outside the assigned manager region.
    pub unmanaged_bytes: u64,
    pub metadata_bytes_read: u64,
    pub memories: Vec<MemoryAllocationEntry>,
}

/// One manager ID's measured capacity and independently sourced ownership metadata.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryAllocationEntry {
    pub memory_manager_id: u8,
    pub binding: MemoryAllocationBinding,
    pub range_claim: Option<MemoryAllocationRangeClaim>,
    pub virtual_extent: MemoryAllocationSizeEntry,
    pub allocated_buckets: u16,
    pub allocated_bytes: u64,
    pub bucket_slack_bytes: u64,
    /// Unavailable: allocation metadata does not measure payload occupancy.
    pub payload_bytes: Option<u64>,
}

/// Current range policy metadata; this does not establish a stable-key binding.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryAllocationRangeClaim {
    pub authority: String,
    pub mode: MemoryRangeAuthorityMode,
}

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
