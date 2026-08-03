//! Passive boundary contracts for the Fleet Subnet Root prepaid Canister pool.

use crate::{
    cdk::types::Cycles,
    ids::{ComponentInstanceId, FleetSubnetCanisterPoolConfig},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Identifies the durable Component allocation that has claimed one empty Canister.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolClaim {
    pub component: Option<ComponentInstanceId>,
    pub operation_id: [u8; 32],
}

/// How one prepaid empty Canister entered the root-owned inventory.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolAssetOrigin {
    Created,
    Imported,
    Recycled,
}

/// Current durable state of one prepaid empty-Canister asset.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolAssetStatus {
    PendingReset,
    Ready,
    Claimed { claim: CanisterPoolClaim },
    HandingOff { recipient: Principal },
    Failed { reason: String },
}

/// Controller-visible inventory row for one prepaid empty Canister.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolAsset {
    pub canister_id: Principal,
    pub cycles: Cycles,
    pub origin: CanisterPoolAssetOrigin,
    pub status: CanisterPoolAssetStatus,
    pub added_at_ns: u64,
    pub updated_at_ns: u64,
}

/// Durable refill creation whose paid effect is incomplete or awaiting commit.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolCreation {
    pub operation_id: [u8; 32],
    pub canister_cycles: Cycles,
    pub canister_id: Option<Principal>,
    pub prepared_at_ns: u64,
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

/// Exact configured policy and current pool inventory.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolResponse {
    pub config: FleetSubnetCanisterPoolConfig,
    pub tracked: u32,
    pub surplus: u32,
    pub ready: u32,
    pub pending_reset: u32,
    pub claimed: u32,
    pub handing_off: u32,
    pub failed: u32,
    pub completed_handoffs: u64,
    pub pending_creation: Option<CanisterPoolCreation>,
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
    Created {
        canister_id: Principal,
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
            surplus: 0,
            ready: 0,
            pending_reset: 0,
            claimed: 0,
            handing_off: 0,
            failed: 1,
            completed_handoffs: 0,
            pending_creation: None,
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
