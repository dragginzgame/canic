//
// ShardingRegistryEntry
//

use crate::dto::prelude::*;

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct ShardingRegistryEntry {
    pub pid: Principal,
    pub entry: ShardEntry,
}

//
// ShardingRegistryResponse
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct ShardingRegistryResponse(pub Vec<ShardingRegistryEntry>);

//
// ShardingPartitionKeysResponse
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct ShardingPartitionKeysResponse(pub Vec<String>);

//
// ShardEntry
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct ShardEntry {
    // Logical slot index within the pool (assigned deterministically).
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
    pub pool: String,
    pub canister_role: CanisterRole,
    pub created_at: u64,
}

//
// ShardingPlanStateResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum ShardingPlanStateResponse {
    // Partition key already has a shard assigned.
    AlreadyAssigned { pid: Principal },

    // Partition key can be deterministically assigned to an existing shard (via HRW).
    UseExisting { pid: Principal },

    // Policy allows creation of a new shard.
    CreateAllowed,

    // Policy forbids creation of a new shard (e.g., capacity reached).
    CreateBlocked { reason: String },
}
