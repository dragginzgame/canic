use crate::{cdk::types::BoundedString64, dto::prelude::*};

///
/// ScalingRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ScalingRegistryView(pub Vec<(Principal, WorkerEntryView)>);

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
/// ShardingRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardingRegistryView(pub Vec<(Principal, ShardEntryView)>);

///
/// ShardingTenantsView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardingTenantsView(pub Vec<String>);

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

///
/// ShardingPlanStateView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ShardingPlanStateView {
    /// Tenant already has a shard assigned.
    AlreadyAssigned { pid: Principal },

    /// Tenant can be deterministically assigned to an existing shard (via HRW).
    UseExisting { pid: Principal },

    /// Policy allows creation of a new shard.
    CreateAllowed,

    /// Policy forbids creation of a new shard (e.g., capacity reached).
    CreateBlocked { reason: String },
}
