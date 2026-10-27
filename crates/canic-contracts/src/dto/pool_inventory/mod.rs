//! Complete native physical pool pages; stored inventory and paid-effect evidence are unchanged.
use crate::{
    cycles::Cycles,
    dto::{
        pool::{CanisterPoolAssetOrigin, CanisterPoolClaim, CanisterPoolRecycleReset},
        pool_census::{Creation, Handoff},
    },
    ids::FleetSubnetCanisterPoolConfig,
};
use ic_platform::Principal;

/// Complete CanisterPoolStatusRequest transport contract.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Request {
    pub start_after: Option<Principal>,
    pub limit: u16,
}

/// Complete CanisterPoolResponse transport contract.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Page {
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
    pub pending_creation: Option<Creation>,
    pub pending_handoff: Option<Handoff>,
    pub entries: Vec<Asset>,
    pub next_start_after: Option<Principal>,
}

/// Complete CanisterPoolAsset transport contract.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Asset {
    pub canister_id: Principal,
    pub creation_receipt: Option<CreationReceipt>,
    pub cycles: Cycles,
    pub origin: CanisterPoolAssetOrigin,
    pub status: AssetStatus,
    pub added_at_ns: u64,
    pub updated_at_ns: u64,
}

/// Complete CanisterPoolCreationReceipt transport contract.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CreationReceipt {
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

/// Complete CanisterPoolAssetStatus transport contract.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum AssetStatus {
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
