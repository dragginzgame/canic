//! Native pool evidence; retained storage and spending authority remain canonical.
use crate::{
    cycles::Cycles,
    dto::{
        pool::CanisterPoolCreationFailure,
        pool_import::{PoolImportPhase, PoolImportRootReceipt},
    },
};
use ic_platform::Principal;

/// Complete RootPoolReleaseResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Census {
    pub root: Principal,
    pub bootstrap: Option<Bootstrap>,
    pub capacity_import: Option<ImportStatus>,
    pub creation: Option<Creation>,
    pub handoff: Option<Handoff>,
}

/// Complete RootPoolBootstrapReleaseEvidence evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Bootstrap {
    pub review_sha256: [u8; 32],
    pub install_id: [u8; 32],
    pub operator: Principal,
    pub store: Principal,
    pub sources: Vec<Principal>,
}

/// Complete PoolImportSource evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Source {
    pub canister_id: Principal,
    /// Actual source custody before any optional operator handoff.
    pub controllers: Vec<Principal>,
    pub module_sha256: Option<[u8; 32]>,
    pub canister_version: u64,
    pub stopped: bool,
    pub disposition_sha256: [u8; 32],
    pub observed_cycles: u128,
    pub observed_reserved_cycles: u128,
    pub minimum_ready_cycles: u128,
    pub maximum_debit_cycles: u128,
}

/// Complete PoolImportReservation evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Reservation {
    pub sequence: u64,
    pub plan_sha256: [u8; 32],
    pub root_authority_sha256: [u8; 32],
    pub root: Principal,
    pub operator: Principal,
    pub subnet: Principal,
    pub transitional_controllers: Vec<Principal>,
    pub final_controllers: Vec<Principal>,
    pub sources: Vec<Source>,
    pub observed_root_cycles: u128,
    pub observed_root_reserved_cycles: u128,
    pub minimum_root_cycles: u128,
    pub maximum_root_debit_cycles: u128,
    pub maximum_paid_calls: u32,
}

/// Complete PoolImportSourceReceipt evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SourceReceipt {
    pub root_sender_canister_version: u64,
    pub canister_id: Principal,
    pub canister_version: u64,
    pub before_uninstall_canister_version: u64,
    pub retained_cycles: u128,
    pub retained_reserved_cycles: u128,
    /// Net deficit from the original native plus reserved balance; zero for net surplus.
    /// Positive retained surplus does not authenticate funding or replenish debit authority.
    pub observed_debit_cycles: u128,
}

/// Complete PoolImportStatus evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ImportStatus {
    pub reserved_at_ns: u64,
    pub reservation: Reservation,
    pub progress: Vec<SourceProgress>,
    pub phase: PoolImportPhase,
    pub paid_calls: u32,
    pub reserved_debit_cycles: u128,
    pub last_root_cycles: u128,
    pub root_receipt: Option<PoolImportRootReceipt>,
}

/// Complete CanisterPoolHandoff evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Handoff {
    pub canister_id: Principal,
    pub recipient: Principal,
    pub prepared_at_ns: u64,
}

/// Complete CanisterPoolCreation evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Creation {
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
    pub progress: CreationProgress,
}

/// Complete PoolImportSourceProgress progress without retry reinterpretation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum SourceProgress {
    AwaitingHandoff,
    ControllersIssued,
    ControllersConfirmed,
    StopIssued,
    Stopped,
    UninstallIssued,
    Ready(SourceReceipt),
}

/// Complete CanisterPoolCreationProgress progress without retry reinterpretation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum CreationProgress {
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
