use crate::dto::prelude::*;

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
}
