use crate::{
    cdk::candid::Principal,
    dto::placement::sharding::{ShardEntry, ShardingPlanStateResponse},
    storage::stable::sharding::{ShardEntryRecord, ShardKey},
    view::placement::sharding::{ShardPlacement, ShardTenantAssignment, ShardingPlanState},
};

///
/// ShardPlacementPolicyInputMapper
///

pub struct ShardPlacementPolicyInputMapper;

impl ShardPlacementPolicyInputMapper {
    #[must_use]
    pub fn record_to_policy_input(
        pid: Principal,
        entry: &ShardEntryRecord,
    ) -> (Principal, ShardPlacement) {
        (
            pid,
            ShardPlacement {
                pool: entry.pool.to_string(),
                slot: entry.slot,
                capacity: entry.capacity,
                count: entry.count,
                role: entry.canister_role.clone(),
                created_at: entry.created_at,
            },
        )
    }
}

///
/// ShardTenantAssignmentPolicyInputMapper
///

pub struct ShardTenantAssignmentPolicyInputMapper;

impl ShardTenantAssignmentPolicyInputMapper {
    #[must_use]
    pub fn record_to_policy_input(key: &ShardKey, pid: Principal) -> ShardTenantAssignment {
        ShardTenantAssignment {
            tenant: key.tenant.to_string(),
            pid,
        }
    }
}

///
/// ShardEntryMapper
///

pub struct ShardEntryMapper;

impl ShardEntryMapper {
    #[must_use]
    pub fn record_to_view(entry: &ShardEntryRecord) -> ShardEntry {
        ShardEntry {
            slot: entry.slot,
            capacity: entry.capacity,
            count: entry.count,
            pool: entry.pool.to_string(),
            canister_role: entry.canister_role.clone(),
            created_at: entry.created_at,
        }
    }
}

///
/// ShardingPlanStateResponseMapper
///

pub struct ShardingPlanStateResponseMapper;

impl ShardingPlanStateResponseMapper {
    #[must_use]
    pub fn record_to_view(state: ShardingPlanState) -> ShardingPlanStateResponse {
        match state {
            ShardingPlanState::AlreadyAssigned { pid } => {
                ShardingPlanStateResponse::AlreadyAssigned { pid }
            }
            ShardingPlanState::UseExisting { pid } => {
                ShardingPlanStateResponse::UseExisting { pid }
            }
            ShardingPlanState::CreateAllowed => ShardingPlanStateResponse::CreateAllowed,
            ShardingPlanState::CreateBlocked { reason } => {
                ShardingPlanStateResponse::CreateBlocked {
                    reason: reason.to_string(),
                }
            }
        }
    }
}
