//
// ScalingRegistryEntry
//

use crate::dto::prelude::*;

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct ScalingRegistryEntry {
    pub pid: Principal,
    pub entry: WorkerEntry,
}

//
// ScalingRegistryResponse
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct ScalingRegistryResponse(pub Vec<ScalingRegistryEntry>);

//
// WorkerEntry
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct WorkerEntry {
    pub pool: String,                // which scale pool this belongs to
    pub canister_role: CanisterRole, // canister role
    pub created_at_secs: u64,        // timestamp
}
