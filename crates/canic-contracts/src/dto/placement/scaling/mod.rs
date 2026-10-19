//
// ScalingRegistryEntry
//

use crate::dto::prelude::*;

#[derive(CandidType, Clone, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct ScalingRegistryEntry {
    pub pid: Principal,
    pub entry: WorkerEntry,
}

//
// ScalingRegistryResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct ScalingRegistryResponse(pub Vec<ScalingRegistryEntry>);

//
// WorkerEntry
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct WorkerEntry {
    pub pool: String,                // which scale pool this belongs to
    pub canister_role: CanisterRole, // canister role
    pub created_at_secs: u64,        // timestamp
}
