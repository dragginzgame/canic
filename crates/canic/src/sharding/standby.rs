//! Root-owned standby inventory and one-use refill approvals.
use crate::method::Contract;
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// Typed reserve outcome with the operator diagnostic for this exact attempt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum ReserveError {
    /// Caller does not own this operation.
    Unauthorized(String),
    /// Request parameters violate the approved rule.
    InvalidApproval(String),
    /// Request differs from the retained approval.
    ApprovalConflict(String),
    /// The original deadline does not admit dispatch.
    Deadline(String),
    /// Root lacks the required operating reserve.
    InsufficientCycles(String),
    /// The approved inventory or subnet ceiling is exhausted.
    Capacity(String),
    /// The operation lacks a prerequisite grant.
    NotConfigured(String),
    /// Another authority or operation retains custody.
    CustodyOccupied(String),
    /// The supplied principal or installation is ineligible.
    WrongAsset(String),
    /// Observe the original operation for confirmation.
    AwaitingConfirmation(String),
    /// The returned evidence could not be verified.
    InvalidObservation(String),
    /// An installation still has outstanding role authority.
    ProofOutstanding(String),
    /// Creation was issued without a confirmed principal.
    RefillIssued(String),
    /// Controller handoff was issued without confirmation.
    HandoffIssued(String),
    /// A retained management effect was issued without confirmation.
    EffectIssued(String),
    /// The candidate and Root belong to different subnets.
    WrongSubnet(String),
    /// The observed balance is below the approved floor.
    BalanceBelowMinimum(String),
}

impl std::fmt::Display for ReserveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized(message)
            | Self::InvalidApproval(message)
            | Self::ApprovalConflict(message)
            | Self::Deadline(message)
            | Self::InsufficientCycles(message)
            | Self::Capacity(message)
            | Self::NotConfigured(message)
            | Self::CustodyOccupied(message)
            | Self::WrongAsset(message)
            | Self::AwaitingConfirmation(message)
            | Self::InvalidObservation(message)
            | Self::ProofOutstanding(message)
            | Self::RefillIssued(message)
            | Self::HandoffIssued(message)
            | Self::EffectIssued(message)
            | Self::WrongSubnet(message)
            | Self::BalanceBelowMinimum(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ReserveError {}

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

/// How Root originally acquired an inventory principal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Source {
    /// Root spent the approved refill debit.
    Refill(Refill),
    /// An existing shard donated its physical slot after verified retirement.
    Shard(super::ShardTarget),
    /// Operator supplied unassigned canister.
    Imported(Principal),
    /// Root created this entry against the fleet reserve allowance.
    Automatic(Refill),
    /// Worker created against its hub’s allowance, then released to the fleet.
    Worker(super::ShardTarget),
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
    /// Reset intent is retained with its original deadline.
    Reset(Principal, super::Allocation, u64, ResetPhase),
    /// Empty and running with this observed balance. Original assignments remain retired.
    Available(Principal, u128),
    /// Registered for handoff; Root has yet to verify custody.
    AwaitingHandoff(Principal),
    /// Former Root has yet to release this principal to the destination.
    AwaitingRelease(Principal),
    /// Verified empty and solely controlled until the recorded proof cutoff.
    Quarantined(Principal, u64),
    /// Pinned to the destination; the source still counts pending transfers.
    Transfer(Principal, Principal, u64, TransferPhase),
}

/// Progress of a source fleet's handoff to one approved destination.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransferPhase {
    /// Removed from the source's claimable inventory.
    Reserved,
    /// Sole-controller change issued once. Only custody observation can settle it.
    ControllersIssued,
    /// Destination reported verified custody. The source has released its slot.
    Confirmed,
}

/// Read the destination inventory entry as its exact configured former Root.
pub struct ObserveStandbyHandoff;
impl Contract for ObserveStandbyHandoff {
    const NAME: &'static str = "observe_standby_handoff";
    const ID: u64 = ic_wire::method_id("canic.placement.observe_standby_handoff.v1");
    type Input = Principal;
    type Output = Result<Asset, ReserveError>;
}

/// Export one exact empty spare to its named destination Root.
pub struct ExportStandby;
impl Contract for ExportStandby {
    const NAME: &'static str = "export_standby";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = (Principal, Principal, u64);
    type Output = Result<TransferPhase, ReserveError>;
}

/// Approve one refill or recover the original result.
pub struct RefillStandby;
impl Contract for RefillStandby {
    const NAME: &'static str = "refill_standby";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = Refill;
    const REPLY: ic_wire::Limits = super::LIMITS;
    type Output = Result<Asset, ReserveError>;
}

/// Read the ordered operation history without dispatching work.
pub struct ObserveStandby;
impl Contract for ObserveStandby {
    const NAME: &'static str = "observe_standby";
    const ID: u64 = ic_wire::method_id("canic.placement.observe_standby.v1");
    const REPLY: ic_wire::Limits = ic_wire::Limits {
        bytes: 16384,
        allocation: 524288,
        nodes: 16384,
        depth: 12,
    };
    type Input = ();
    type Output = Vec<(Source, Asset)>;
}

/// Verify an approved external handoff or complete its quarantine.
pub struct ImportStandby;
impl Contract for ImportStandby {
    const NAME: &'static str = "import_standby";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = Principal;
    type Output = Result<Asset, ReserveError>;
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
    type Output = Result<(), ReserveError>;
}

/// Durable reset progress. Issued calls are reconciled by management observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ResetPhase {
    /// Stop was issued; the target must be observed stopped.
    StopIssued,
    /// Uninstall was issued; the target must be observed empty.
    UninstallIssued,
    /// Empty code and snapshot inventory observed. Balance verification precedes restart.
    Uninstalled,
    /// Start issued for the uninstalled principal. Reconciliation checks live status.
    StartIssued,
    /// The original generation has been permanently returned.
    Returned,
}

/// Discard the released generation's code under an exact expiring approval.
pub struct ResetWorker;
impl Contract for ResetWorker {
    const NAME: &'static str = "reset_worker";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = super::LIMITS;
    type Input = (super::ShardTarget, u64);
    type Output = Result<ResetPhase, ReserveError>;
}
