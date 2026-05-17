use crate::dto::prelude::*;

///
/// MemoryLedgerResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryLedgerResponse {
    pub magic: u64,
    pub format_id: u32,
    pub schema_version: u32,
    pub header_len: u32,
    pub header_checksum: u64,
    pub current_generation: u64,
    pub authorities: Vec<MemoryRangeAuthorityEntry>,
    pub ranges: Vec<MemoryRangeEntry>,
    pub entries: Vec<MemoryRegistryEntry>,
}

///
/// MemoryRangeAuthorityEntry
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryRangeAuthorityEntry {
    pub owner: String,
    pub start: u8,
    pub end: u8,
    pub purpose: String,
}

///
/// MemoryRangeEntry
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryRangeEntry {
    pub owner: String,
    pub start: u8,
    pub end: u8,
}

///
/// MemoryRegistryResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryRegistryResponse {
    pub entries: Vec<MemoryRegistryEntry>,
}

///
/// MemoryRegistryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryRegistryEntry {
    pub id: u8,
    pub crate_name: String,
    pub label: String,
    pub stable_key: String,
    pub schema_version: Option<u32>,
    pub schema_fingerprint: Option<String>,
}
