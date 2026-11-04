//! Passive observations of Storage-owned allocations and directory capacity.

use crate::dto::prelude::*;

/// Current named allocations with physical capacity totals from the same owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MemoryAllocationsResponse {
    pub summary: StorageDirectorySummary,
    pub memories: Vec<MemoryAllocationEntry>,
}

/// One retained allocation; capacity is not application payload occupancy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MemoryAllocationEntry {
    pub allocation_id: u64,
    pub stable_key: String,
    pub capacity: MemoryAllocationSizeEntry,
}

/// Validated physical directory accounting, without keys or allocation authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StorageDirectorySummary {
    pub generation: u64,
    pub allocations: u64,
    pub admitted_allocations: u64,
    pub live_leases: u64,
    pub extents: u64,
    pub data_pages: u64,
    pub metadata_pages: u64,
    pub directory_bytes: u64,
    pub physical_pages: u64,
    pub unassigned_pages: u64,
}

/// Logical capacity of one admitted allocation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MemoryAllocationSizeEntry {
    pub wasm_pages: u64,
    pub bytes: u64,
}
