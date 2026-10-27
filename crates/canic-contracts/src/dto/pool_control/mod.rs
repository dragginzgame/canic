//! Physical pool handoff boundary; the existing pool owner performs the transfer.
use ic_platform::Principal;
use serde::{Deserialize, Serialize};
/// Exact canister and recipient selected by its Root controller.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct HandoffRequest {
    pub canister_id: Principal,
    pub recipient: Principal,
}
/// Confirmed exact physical handoff.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct HandoffResponse {
    pub canister_id: Principal,
    pub recipient: Principal,
}

/// One maintenance step, including retained uncertain creation evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum MaintenanceResponse {
    Maintained,
    MaintenancePaused {
        reason: String,
    },
    Created {
        canister_id: Principal,
    },
    RefillWaitingForCycles {
        available: crate::cycles::Cycles,
        attempt_count: u32,
        creation_amount: crate::cycles::Cycles,
        execution_margin: crate::cycles::Cycles,
        last_attempt_at_ns: Option<u64>,
        ledger_fee: crate::cycles::Cycles,
        readiness_floor: crate::cycles::Cycles,
        required: crate::cycles::Cycles,
        retry_at_ns: u64,
        shortfall: crate::cycles::Cycles,
    },
    RefillPending {
        operation_id: [u8; 32],
        uncertain_result: bool,
    },
    RefillBlocked {
        operation_id: [u8; 32],
        failure: crate::dto::pool::CanisterPoolCreationFailure,
    },
    ResetReady {
        canister_id: Principal,
    },
    ResetFailed {
        canister_id: Principal,
        reason: String,
    },
}

/// Existing physical canister selected for the Root-owned reset/import workflow.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ImportRequest {
    pub canister_id: Principal,
}
