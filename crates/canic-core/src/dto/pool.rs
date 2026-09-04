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
    Created,
    Imported,
    Recycled,
}

/// Exact Cycles Ledger receipt retained for one autonomously created pool asset.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolCreationReceipt {
    pub block_index: u64,
    pub operation_id: [u8; 32],
    pub cycles_ledger: Principal,
    pub ledger_amount: Cycles,
    pub ledger_fee: Cycles,
    pub readiness_floor: Cycles,
    pub creation_execution_margin: Cycles,
    pub management_creation_fee: Cycles,
    pub first_observed_cycles: Option<Cycles>,
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
    pub creation_receipt: Option<CanisterPoolCreationReceipt>,
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

/// Why one autonomous refill stopped without creating another Canister.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolCreationFailure {
    UnresolvedAfterLedgerWindow,
    LedgerCreationFailed,
    LedgerRejected,
}

/// Durable controller-visible progress of one autonomous refill.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolCreationProgress {
    Intent {
        uncertain_result: bool,
    },
    Created {
        block_index: u64,
        canister_id: Principal,
    },
    WaitingForFunding {
        available: Cycles,
        attempt_count: u32,
        last_attempt_at_ns: Option<u64>,
        observed_at_ns: u64,
        required: Cycles,
        retry_at_ns: u64,
        shortfall: Cycles,
    },
    Blocked {
        failure: CanisterPoolCreationFailure,
    },
}

/// Exact Cycles Ledger request retained until its principal is in inventory.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterPoolCreation {
    pub attempt_count: u32,
    pub operation_id: [u8; 32],
    pub cycles_ledger: Principal,
    pub placement_subnet: Principal,
    pub root: Principal,
    pub ledger_amount: Cycles,
    pub ledger_fee: Cycles,
    pub readiness_floor: Cycles,
    pub creation_execution_margin: Cycles,
    pub management_creation_fee: Cycles,
    pub created_at_time_ns: u64,
    pub last_attempt_at_ns: Option<u64>,
    pub progress: CanisterPoolCreationProgress,
}

/// Bounded controller query for one canonical inventory page.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct CanisterPoolStatusRequest {
    pub start_after: Option<Principal>,
    pub limit: u16,
}

/// Selects one pool Canister for an import or reset retry command.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct PoolCanisterRequest {
    pub canister_id: Principal,
}

/// Selects one pool Canister and its exact handoff recipient.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct PoolHandoffRequest {
    pub canister_id: Principal,
    pub recipient: Principal,
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
    pub pending_creation: Option<CanisterPoolCreation>,
    pub pending_handoff: Option<CanisterPoolHandoff>,
    pub entries: Vec<CanisterPoolAsset>,
    pub next_start_after: Option<Principal>,
}

/// Controller command for explicit pool maintenance.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminCommand {
    Maintain,
    RetryRefill,
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
    RefillWaitingForCycles {
        available: Cycles,
        attempt_count: u32,
        creation_amount: Cycles,
        execution_margin: Cycles,
        last_attempt_at_ns: Option<u64>,
        ledger_fee: Cycles,
        readiness_floor: Cycles,
        required: Cycles,
        retry_at_ns: u64,
        shortfall: Cycles,
    },
    RefillPending {
        operation_id: [u8; 32],
        uncertain_result: bool,
    },
    RefillBlocked {
        operation_id: [u8; 32],
        failure: CanisterPoolCreationFailure,
    },
    RefillRetryScheduled {
        previous_operation_id: [u8; 32],
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

/// Narrow result of one explicit maintenance pass.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolMaintenanceResponse {
    Maintained,
    MaintenancePaused {
        reason: String,
    },
    Created {
        canister_id: Principal,
    },
    RefillWaitingForCycles {
        available: Cycles,
        attempt_count: u32,
        creation_amount: Cycles,
        execution_margin: Cycles,
        last_attempt_at_ns: Option<u64>,
        ledger_fee: Cycles,
        readiness_floor: Cycles,
        required: Cycles,
        retry_at_ns: u64,
        shortfall: Cycles,
    },
    RefillPending {
        operation_id: [u8; 32],
        uncertain_result: bool,
    },
    RefillBlocked {
        operation_id: [u8; 32],
        failure: CanisterPoolCreationFailure,
    },
    ResetReady {
        canister_id: Principal,
    },
    ResetFailed {
        canister_id: Principal,
        reason: String,
    },
}

/// Narrow result of importing one existing physical Canister.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolImportResponse {
    Imported {
        canister_id: Principal,
    },
    ResetFailed {
        canister_id: Principal,
        reason: String,
    },
}

/// Exact result of scheduling another blocked refill attempt.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct PoolRefillRetryResponse {
    pub previous_operation_id: [u8; 32],
}

/// Exact result of scheduling another reset attempt.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct PoolResetRetryResponse {
    pub canister_id: Principal,
}

/// Exact result of handing one physical Canister to replacement authority.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct PoolHandoffResponse {
    pub canister_id: Principal,
    pub recipient: Principal,
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
                creation_execution_margin: Cycles::new(1_000_000_000_000),
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
            pending_creation: Some(CanisterPoolCreation {
                attempt_count: 1,
                operation_id: [8; 32],
                cycles_ledger: Principal::from_slice(&[6; 29]),
                placement_subnet: Principal::from_slice(&[5; 29]),
                root: Principal::from_slice(&[4; 29]),
                ledger_amount: Cycles::new(6_500_000_000_000),
                ledger_fee: Cycles::new(100_000_000),
                readiness_floor: Cycles::new(5_000_000_000_000),
                creation_execution_margin: Cycles::new(1_000_000_000_000),
                management_creation_fee: Cycles::new(500_000_000_000),
                created_at_time_ns: 12,
                last_attempt_at_ns: Some(13),
                progress: CanisterPoolCreationProgress::Blocked {
                    failure: CanisterPoolCreationFailure::LedgerCreationFailed,
                },
            }),
            pending_handoff: None,
            entries: vec![CanisterPoolAsset {
                canister_id,
                creation_receipt: None,
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

        let command = PoolAdminCommand::RetryRefill;
        let bytes = candid::encode_one(&command).expect("encode refill retry command");
        assert_eq!(
            candid::decode_one::<PoolAdminCommand>(&bytes).expect("decode refill retry command"),
            command,
        );
    }
}
