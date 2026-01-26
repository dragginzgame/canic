use crate::dto::prelude::*;

///
/// ShardingRegistryEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardingRegistryEntry {
    pub pid: Principal,
    pub entry: ShardEntry,
}

///
/// ShardingRegistryResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardingRegistryResponse(pub Vec<ShardingRegistryEntry>);

///
/// ShardingTenantsResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardingTenantsResponse(pub Vec<String>);

///
/// ShardEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShardEntry {
    /// Logical slot index within the pool (assigned deterministically).
    pub slot: u32,
    pub capacity: u32,
    pub count: u32,
    pub pool: String,
    pub canister_role: CanisterRole,
    pub created_at: u64,
}

///
/// ShardingPlanStateResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ShardingPlanStateResponse {
    /// Tenant already has a shard assigned.
    AlreadyAssigned { pid: Principal },

    /// Tenant can be deterministically assigned to an existing shard (via HRW).
    UseExisting { pid: Principal },

    /// Policy allows creation of a new shard.
    CreateAllowed,

    /// Policy forbids creation of a new shard (e.g., capacity reached).
    CreateBlocked { reason: String },
}

///
/// ShardingAdminCommand
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum ShardingAdminCommand {
    RegisterShardCreated { pid: Principal },
    MarkShardProvisioned { pid: Principal },
    AdmitShardToHrw { pid: Principal },
    RetireShard { pid: Principal },
    RevokeShard { pid: Principal },
}

///
/// ShardingAdminResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum ShardingAdminResponse {
    Registered { pid: Principal },
    Provisioned { pid: Principal },
    Admitted { pid: Principal },
    Retired { pid: Principal },
    Revoked { pid: Principal },
}
