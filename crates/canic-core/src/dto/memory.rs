use crate::dto::prelude::*;

///
/// MemoryRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MemoryRegistryView {
    pub entries: Vec<MemoryRegistryEntryView>,
}

///
/// MemoryRegistryEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MemoryRegistryEntryView {
    pub id: u8,
    pub crate_name: String,
    pub label: String,
}
