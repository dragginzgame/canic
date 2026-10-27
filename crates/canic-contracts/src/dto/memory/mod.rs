//! Module: dto::memory
//!
//! Responsibility: expose passive current memory diagnostic contracts.
//! Does not own: allocation state, ledger storage, or diagnostic collection.
//! Boundary: ops projects memory state into typed native observations.

use crate::dto::prelude::*;

pub use crate::values::memory::{MemoryAllocationBinding, MemoryCommitRecoveryErrorResponse};

/// Measured physical and virtual allocations exposed by protected observations.
/// Covers all usable IDs, including unknown bindings and the substrate ledger.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
pub struct MemoryCommitRecoveryResponse {
    pub current: MemoryCommitSnapshotResponse,
    pub authoritative_generation: Option<u64>,
    pub recovery_error: Option<MemoryCommitRecoveryErrorResponse>,
}

///
/// MemoryCommitSnapshotResponse
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryCommitSnapshotResponse {
    pub present: bool,
    pub generation: Option<u64>,
    pub valid: bool,
}

/// Current host grants and unmanaged physical exclusions, including governance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryAllocationPoolEntry {
    pub authorities: Vec<MemoryAuthorityEntry>,
    pub excluded_ids: Vec<u8>,
}

/// One admitted linked-code owner and its permanent key namespace.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryAuthorityEntry {
    pub owner: String,
    pub key_prefix: String,
}

///
/// MemoryLedgerMemoryEntry
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct MemoryLedgerMemoryEntry {
    pub memory_manager_id: u8,
    pub stable_key: String,
    pub size: MemoryAllocationSizeEntry,
}

///
/// MemoryAllocationRecordEntry
///

#[derive(Clone, Debug, Deserialize)]
pub struct MemoryAllocationRecordEntry {
    pub memory_manager_id: Option<u8>,
    pub stable_key: String,
    pub memory_size: Option<MemoryAllocationSizeEntry>,
}

///
/// MemoryAllocationSizeEntry
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MemoryAllocationSizeEntry {
    pub wasm_pages: u64,
    pub bytes: u64,
}
