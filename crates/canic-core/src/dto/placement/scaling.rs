use crate::dto::prelude::*;

///
/// ScalingRegistryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ScalingRegistryEntry {
    pub pid: Principal,
    pub entry: WorkerEntry,
}

///
/// ScalingRegistryResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ScalingRegistryResponse(pub Vec<ScalingRegistryEntry>);

///
/// WorkerEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerEntry {
    pub pool: String,                // which scale pool this belongs to
    pub canister_role: CanisterRole, // canister role
    pub created_at_secs: u64,        // timestamp
}
