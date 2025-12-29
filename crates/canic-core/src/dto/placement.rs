use crate::{cdk::types::BoundedString64, dto::prelude::*};

///
/// ScalingRegistryView
///

pub type ScalingRegistryView = Vec<(Principal, WorkerEntryView)>;

///
/// ShardingRegistryView
///

pub type ShardingRegistryView = Vec<(Principal, ShardEntryView)>;

///
/// WorkerEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerEntryView {
    pub pool: BoundedString64,       // which scale pool this belongs to
    pub canister_role: CanisterRole, // canister role
    pub created_at_secs: u64,        // timestamp
}

///
/// ShardEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShardEntryView {
    /// Logical slot index within the pool (assigned deterministically).
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
    pub pool: BoundedString64,
    pub canister_role: CanisterRole,
    pub created_at: u64,
}
