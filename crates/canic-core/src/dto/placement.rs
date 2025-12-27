use crate::dto::prelude::*;

///
/// ScalingRegistryView
///

pub type ScalingRegistryView = Vec<(Principal, WorkerEntryView)>;

///
/// WorkerEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerEntryView {
    pub pool: BoundedString64,       // which scale pool this belongs to
    pub canister_role: CanisterRole, // canister role
    pub created_at_secs: u64,        // timestamp
}
