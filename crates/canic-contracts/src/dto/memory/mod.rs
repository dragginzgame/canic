//! Module: dto::memory
//!
//! Responsibility: expose passive current memory diagnostic contracts.
//! Does not own: allocation state, ledger storage, or diagnostic collection.
//! Boundary: ops projects the memory runtime into these Candid response shapes.

use crate::dto::prelude::*;

pub use crate::values::memory::{MemoryAllocationBinding, MemoryCommitRecoveryErrorResponse};

/// Measured physical and virtual allocations exposed by protected observations.
/// Covers all usable IDs, including unknown bindings and the substrate ledger.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
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
#[candid_path("ic_platform::candid::contract")]
pub struct MemoryAllocationEntry {
    pub memory_manager_id: u8,
    pub binding: MemoryAllocationBinding,
    /// Host pool eligibility does not imply that the slot is free.
    pub pool_eligible: Option<bool>,
    pub virtual_extent: MemoryAllocationSizeEntry,
    pub allocated_buckets: u16,
    pub allocated_bytes: u64,
    pub bucket_slack_bytes: u64,
    /// Unavailable: allocation metadata does not measure payload occupancy.
    pub payload_bytes: Option<u64>,
}

///
/// MemoryLedgerResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct MemoryLedgerResponse {
    pub ledger_schema_version: u32,
    pub physical_format_id: u32,
    pub current_generation: u64,
    /// Exact protected ledger slot in the selected memory runtime.
    pub ledger_memory_manager_id: u8,
    pub commit_recovery: MemoryCommitRecoveryResponse,
    pub allocation_pool: MemoryAllocationPoolEntry,
    pub memories: Vec<MemoryLedgerMemoryEntry>,
    pub records: Vec<MemoryAllocationRecordEntry>,
}

///
/// MemoryCommitRecoveryResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct MemoryCommitRecoveryResponse {
    pub current: MemoryCommitSnapshotResponse,
    pub authoritative_generation: Option<u64>,
    pub recovery_error: Option<MemoryCommitRecoveryErrorResponse>,
}

///
/// MemoryCommitSnapshotResponse
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct MemoryCommitSnapshotResponse {
    pub present: bool,
    pub generation: Option<u64>,
    pub valid: bool,
}

/// Current host grants and unmanaged physical exclusions, including governance.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct MemoryAllocationPoolEntry {
    pub authorities: Vec<MemoryAuthorityEntry>,
    pub excluded_ids: Vec<u8>,
}

/// One admitted linked-code owner and its permanent key namespace.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct MemoryAuthorityEntry {
    pub owner: String,
    pub key_prefix: String,
}

///
/// MemoryLedgerMemoryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct MemoryLedgerMemoryEntry {
    pub memory_manager_id: u8,
    pub stable_key: String,
    pub size: MemoryAllocationSizeEntry,
}

///
/// MemoryAllocationRecordEntry
///

#[derive(CandidType, Clone, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct MemoryAllocationRecordEntry {
    pub memory_manager_id: Option<u8>,
    pub stable_key: String,
    pub memory_size: Option<MemoryAllocationSizeEntry>,
}

///
/// MemoryAllocationSizeEntry
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct MemoryAllocationSizeEntry {
    pub wasm_pages: u64,
    pub bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_platform::candid::contract::{Decode, Encode};
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[test]
    fn memory_enums_roundtrip_candid_with_existing_variant_labels() {
        assert_enum_candid_contract(MemoryCommitRecoveryErrorResponse::InvalidSnapshot);
        assert_enum_candid_contract(MemoryCommitRecoveryErrorResponse::UnexpectedGeneration);
    }

    #[test]
    fn allocation_record_roundtrips_permanent_binding_and_measurement() {
        let value = MemoryAllocationRecordEntry {
            memory_manager_id: Some(100),
            stable_key: "app.users.v1".into(),
            memory_size: Some(MemoryAllocationSizeEntry {
                wasm_pages: 2,
                bytes: 131_072,
            }),
        };
        let bytes = Encode!(&value).unwrap();
        let decoded = Decode!(&bytes, MemoryAllocationRecordEntry).unwrap();
        assert_eq!(decoded.memory_manager_id, value.memory_manager_id);
        assert_eq!(decoded.stable_key, value.stable_key);
        assert_eq!(decoded.memory_size, value.memory_size);
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
