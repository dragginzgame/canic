use crate::dto::prelude::*;

//
// DirectoryRegistryEntry
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DirectoryRegistryEntry {
    pub pool: String,
    pub key_value: String,
    pub status: DirectoryEntryStatusResponse,
}

//
// DirectoryRegistryResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct DirectoryRegistryResponse(pub Vec<DirectoryRegistryEntry>);

//
// DirectoryEntryStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum DirectoryEntryStatusResponse {
    Pending {
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
}
