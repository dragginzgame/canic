use crate::dto::prelude::*;

///
/// ShardingRegistryEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardingRegistryEntryView {
    pub pid: Principal,
    pub entry: ShardEntryView,
}

///
/// ShardingRegistryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardingRegistryView(pub Vec<ShardingRegistryEntryView>);

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
    pub pool: String,
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
