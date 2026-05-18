use crate::dto::prelude::*;

///
/// MemoryLedgerResponse
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MemoryLedgerResponse {
    pub magic: u64,
    pub format_id: u32,
    pub schema_version: u32,
    pub layout_epoch: u32,
    pub header_len: u32,
    pub header_checksum: u64,
    pub current_generation: u64,
    pub commit_recovery: MemoryCommitRecoveryResponse,
    pub authorities: Vec<MemoryRangeAuthorityEntry>,
    pub ranges: Vec<MemoryRangeEntry>,
    pub entries: Vec<MemoryRegistryEntry>,
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
/// MemoryCommitRecoveryErrorResponse
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MemoryCommitRecoveryErrorResponse {
    NoValidGeneration,
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
