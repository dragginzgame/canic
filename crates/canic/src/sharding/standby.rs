//! Root-owned standby inventory and one-use refill approvals.
use crate::method::Contract;
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// Exact authorization for one new empty canister on the addressed Root.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Refill {
    /// Principal that will control the empty canister.
    pub root: Principal,
    /// Zero-based, append-only operation number.
    pub sequence: u8,
    /// Maximum cycles attached to creation.
    pub cycles: u64,
    /// Last permitted dispatch time, in IC nanoseconds.
    pub expires_at_ns: u64,
}

/// Original outcome of one refill. Issued entries continue to consume capacity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Asset {
    /// Creation issued without a retained principal.
    Issued,
    /// Created empty, still held exclusively by Root.
    Empty(Principal),
    /// Reserved for the exact allocation, before any install attempt.
    Claimed(Principal, super::Allocation),
    /// The original hub has permanently relinquished dispatch. Cleanup is pending.
    Released(Principal, super::Allocation),
}

/// Approve one refill or recover the original result.
pub struct RefillStandby;
impl Contract for RefillStandby {
    const NAME: &'static str = "refill_standby";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = Refill;
    const REPLY: ic_wire::Limits = super::LIMITS;
    type Output = Result<Asset, String>;
}

/// Read the ordered operation history without dispatching work.
pub struct InspectStandby;
impl Contract for InspectStandby {
    const NAME: &'static str = "inspect_standby";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REPLY: ic_wire::Limits = ic_wire::Limits {
        bytes: 4096,
        allocation: 65536,
        nodes: 4096,
        depth: 12,
    };
    type Input = ();
    type Output = Vec<(Refill, Asset)>;
}

/// Read the hub's retained receipt before cleaning its former worker.
pub struct WorkerReleased;
impl Contract for WorkerReleased {
    const NAME: &'static str = "worker_released";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = super::ShardTarget;
    type Output = bool;
}

/// Ask Root to observe the original worker hub's exact release receipt.
pub struct ConfirmRelease;
impl Contract for ConfirmRelease {
    const NAME: &'static str = "confirm_worker_release";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = super::ShardTarget;
    type Output = Result<(), String>;
}
