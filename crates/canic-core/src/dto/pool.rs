//! Passive boundary contracts for the Fleet Subnet Root physical-Canister inventory.

use crate::{
    cdk::types::Cycles,
    ids::{ComponentInstanceId, FleetSubnetCanisterPoolConfig},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Identifies the durable Component allocation that has claimed one empty Canister.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolClaim {
    pub component: ComponentInstanceId,
    pub operation_id: [u8; 32],
}

/// Reset outcome retained while a stopped workload is still Registry-owned.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolRecycleReset {
    Pending,
    Ready,
    Failed { reason: String },
}

/// How one physical Canister entered the root-owned inventory.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolAssetOrigin {
    InfrastructureStore,
    Imported,
    Recycled,
}

/// Current durable state of one root-owned physical Canister.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolAssetStatus {
    Store,
    StoreDeletionPending {
        operation_id: [u8; 32],
    },
    PendingReset,
    Ready,
    Claimed {
        claim: CanisterPoolClaim,
    },
    Workload {
        claim: CanisterPoolClaim,
    },
    Recycling {
        claim: CanisterPoolClaim,
        reset: CanisterPoolRecycleReset,
    },
    HandingOff {
        recipient: Principal,
    },
    Failed {
        reason: String,
    },
}

/// Controller-visible inventory row for one root-owned physical Canister.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolAsset {
    pub canister_id: Principal,
    pub cycles: Cycles,
    pub origin: CanisterPoolAssetOrigin,
    pub status: CanisterPoolAssetStatus,
    pub added_at_ns: u64,
    pub updated_at_ns: u64,
}

/// Durable transfer of one paid asset to replacement authority during root draining.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolHandoff {
    pub canister_id: Principal,
    pub recipient: Principal,
    pub prepared_at_ns: u64,
}

/// Bounded controller query for one canonical inventory page.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct CanisterPoolStatusRequest {
    pub start_after: Option<Principal>,
    pub limit: u16,
}

/// Exact pool policy and current exclusive root-owned physical inventory.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolResponse {
    pub config: FleetSubnetCanisterPoolConfig,
    pub tracked: u32,
    pub store: u32,
    pub store_deletion_pending: u32,
    pub pooled: u32,
    pub workload: u32,
    pub surplus: u32,
    pub ready: u32,
    pub pending_reset: u32,
    pub claimed: u32,
    pub recycling: u32,
    pub handing_off: u32,
    pub failed: u32,
    pub completed_handoffs: u64,
    pub pending_handoff: Option<CanisterPoolHandoff>,
    pub entries: Vec<CanisterPoolAsset>,
    pub next_start_after: Option<Principal>,
}

/// Controller command for explicit pool maintenance.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminCommand {
    Maintain,
    Import {
        canister_id: Principal,
    },
    RetryReset {
        canister_id: Principal,
    },
    Handoff {
        canister_id: Principal,
        recipient: Principal,
    },
}

/// Result of one explicit pool maintenance command.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminResponse {
    Maintained,
    MaintenancePaused {
        reason: String,
    },
    ReplenishmentRequired {
        ready: u32,
        minimum_size: u32,
        import_capacity: u32,
    },
    Imported {
        canister_id: Principal,
    },
    ResetQueued {
        canister_id: Principal,
    },
    ResetReady {
        canister_id: Principal,
    },
    HandedOff {
        canister_id: Principal,
        recipient: Principal,
    },
    ResetFailed {
        canister_id: Principal,
        reason: String,
    },
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_status_and_admin_contracts_round_trip_through_candid() {
        let canister_id = Principal::from_slice(&[7; 29]);
        let response = CanisterPoolResponse {
            config: FleetSubnetCanisterPoolConfig {
                minimum_size: 3,
                maximum_size: 10,
                canister_cycles: Cycles::new(5_000_000_000_000),
            },
            tracked: 1,
            store: 0,
            store_deletion_pending: 0,
            pooled: 1,
            workload: 0,
            surplus: 0,
            ready: 0,
            pending_reset: 0,
            claimed: 0,
            recycling: 0,
            handing_off: 0,
            failed: 1,
            completed_handoffs: 0,
            pending_handoff: None,
            entries: vec![CanisterPoolAsset {
                canister_id,
                cycles: Cycles::new(4_000_000_000_000),
                origin: CanisterPoolAssetOrigin::Recycled,
                status: CanisterPoolAssetStatus::Failed {
                    reason: "below configured cycles".to_string(),
                },
                added_at_ns: 10,
                updated_at_ns: 11,
            }],
            next_start_after: None,
        };
        let bytes = candid::encode_one(&response).expect("encode pool status");
        assert_eq!(
            candid::decode_one::<CanisterPoolResponse>(&bytes).expect("decode pool status"),
            response,
        );

        let command = PoolAdminCommand::Import { canister_id };
        let bytes = candid::encode_one(&command).expect("encode pool command");
        assert_eq!(
            candid::decode_one::<PoolAdminCommand>(&bytes).expect("decode pool command"),
            command,
        );
    }
}
