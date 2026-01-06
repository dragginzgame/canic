use crate::dto::prelude::*;

///
/// ScalingRegistryEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ScalingRegistryEntryView {
    pub pid: Principal,
    pub entry: WorkerEntryView,
}

///
/// ScalingRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ScalingRegistryView(pub Vec<ScalingRegistryEntryView>);

///
/// WorkerEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerEntryView {
    pub pool: String,                // which scale pool this belongs to
    pub canister_role: CanisterRole, // canister role
    pub created_at_secs: u64,        // timestamp
}
