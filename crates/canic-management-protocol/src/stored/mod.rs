//! Store-backed authority; legacy inline approvals keep their original wire representation.

#[cfg(test)]
mod tests;

use crate::children;
use ic_wasm_store_protocol::{Manifest, Pin};
use serde::{Deserialize, Serialize};

/// Admit Store-backed child creation.
pub const STAGE: u64 = 41;
/// Reconcile creation with its exact Store binding.
pub const RECONCILE: u64 = 42;
/// Admit a Store-backed child upgrade.
pub const STAGE_UPGRADE: u64 = 43;
/// Reconcile an upgrade with its exact Store binding.
pub const RECONCILE_UPGRADE: u64 = 44;
/// Complete journals and artifact retention evidence.
pub const CUSTODY: u64 = 45;
/// Bounded metadata for permanent services and their eight retained upgrades each.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 262_144,
    allocation: 16_777_216,
    nodes: 1_048_576,
    depth: 24,
};

/// Exact published content and its enduring supplier.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reference {
    /// Canonical Store canister principal, on the owner's subnet.
    pub store: String,
    /// Immutable release and ordered content commitments; never “latest”.
    pub manifest: Manifest,
}

/// Approval and artifact supplier form one immutable operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stage<A> {
    /// Existing effect authority, unchanged in its legacy encoding.
    pub approval: A,
    /// Retained before any effect is dispatched.
    pub reference: Reference,
}
impl<A: Serialize> Stage<A> {
    /// Domain-separated identity for the caller-owned Store pin.
    /// # Panics
    /// Panics if the caller supplies metadata outside the bounded contract.
    #[must_use]
    pub fn pin(&self, method: u64, slot: children::ChildId) -> Pin {
        let bytes = ic_wire::encode(&(method, slot, self), ic_wire::Encoding::Payload, LIMITS)
            .expect("bounded Store operation");
        Pin {
            operation: crate::digest(&bytes),
            manifest: self.reference.manifest.clone(),
        }
    }
}

/// Whether durable retention evidence has committed at the effect owner.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PinState {
    /// No retention request dispatched.
    #[default]
    Pending,
    /// Request committed; reply may have been lost.
    Issued,
    /// Exact Store receipt retained or reconstructed through replicated observation.
    Retained,
}
/// Bounded artifact delivery evidence retained independently of the management journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Delivery {
    /// Exact supplier and content.
    pub reference: Reference,
    /// Digest of the complete operation; the manifest has one retained owner above.
    pub operation: [u8; 32],
    /// Retention progress; an uncertain request is never blindly reissued.
    pub state: PinState,
    /// Original cache preparation deadline, retained with the outgoing call.
    pub cache_deadline: Option<u64>,
}
impl Delivery {
    /// Reconstruct the exact Store request from retained operation and manifest.
    #[must_use]
    pub fn pin(&self) -> Pin {
        Pin {
            operation: self.operation,
            manifest: self.reference.manifest.clone(),
        }
    }
}
/// One child's Store evidence, including all admitted upgrade sources.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sources {
    /// Creation source.
    pub creation: Delivery,
    /// Upgrade sources in the same order as the complete upgrade journal.
    pub upgrades: Vec<Delivery>,
}
/// Complete maintenance evidence. Legacy custody methods refuse Store-backed children.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Custody {
    /// Complete creation and upgrade journals.
    pub children: children::Custody,
    /// None denotes an inline child; no raw bytes are exported.
    pub sources: Vec<Option<Sources>>,
}

/// Controller-only approval for Root-managed shard growth from one immutable release.
/// The selected hub is independent of Root's two managed child identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShardPool {
    /// Exact application-owned initialization bytes for each created shard.
    pub arguments: crate::application::Arguments,
    /// Exact effect owner receiving this approval.
    pub root: String,
    /// Stable logical pool name used by legacy HRW slot selection.
    pub pool: String,
    /// Only this canister may request or observe private account placement.
    pub hub: String,
    /// Maximum admitted children, within Root's qualified child bound.
    pub max_shards: u8,
    /// Lifetime ceiling on requester charges for fresh and reserve acquisitions.
    pub creation_budget: u64,
    /// Last time a new child management effect may be dispatched.
    pub expires_at_ns: u64,
    /// Approved release and supplier.
    pub reference: Reference,
}

/// Controller approval for dedicated instances owned by one project hub.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstancePool {
    /// Installing Root and effect owner.
    pub root: String,
    /// Only this parent may claim a child for a retained key.
    pub hub: String,
    /// Declared child component.
    pub component: String,
    /// Approved release and supplier.
    pub reference: Reference,
    /// Exact installation frame containing the approved Root, parent and application arguments.
    pub arguments: crate::application::Arguments,
    /// Maximum distinct dedicated children.
    pub max_instances: u8,
    /// Lifetime ceiling on requester charges for fresh and reserve acquisitions.
    pub creation_budget: u64,
    /// Deadline for issuing new work.
    pub expires_at_ns: u64,
}

/// One controller-approved child role for every ready project of this registry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SingletonGrant {
    /// Registry principal owning the eligible parent allocations.
    pub registry: String,
    /// Child role installed under the requesting project.
    pub role: String,
    /// Store and immutable release.
    pub reference: Reference,
    /// Application bytes nested under the project topology binding.
    pub application: crate::application::Arguments,
    /// Maximum admitted children across the registry.
    pub maximum: u32,
    /// Lifetime ceiling on requester charges for fresh and reserve acquisitions.
    pub creation_budget: u64,
    /// Deadline for new claims.
    pub expires_at_ns: u64,
    /// Approved Auth install frame with an empty payload, filled after the parent is known.
    #[serde(default)]
    pub receiver: Option<crate::application::Arguments>,
}

/// Maximum reserve entries in each acquisition source of the bounded reserve ledger.
pub const MAX_RESERVE_ENTRIES: u8 = 8;
/// Dedicated instances also consume the shared subnet allowance.
pub const MAX_INSTANCES: u8 = crate::children::ChildId::INSTANCES.len() as u8;
/// Addressable shared shard slots.
pub const MAX_SHARDS: u8 = crate::children::ChildId::SHARDS.len() as u8;

/// Aggregate fleet reserve observation, without principals or handoff records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SpareCounts {
    /// Root's approved target.
    pub min_available_spares: u8,
    /// Immediately claimable principals.
    pub ready: u8,
    /// Automatic creations charged, including unresolved attempts.
    pub creations_used: u8,
    /// Lifetime paid creation allowance.
    pub maximum_creations: u8,
    /// Current eligibility for the next automatic refill.
    pub refill: RefillStatus,
}

/// First applicable condition controlling reserve demand.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RefillStatus {
    /// Enough spares for the target.
    AtTarget,
    /// Issued creations cover the remaining deficit.
    AwaitingCreation,
    /// Lifetime count exhausted.
    AllowanceSpent,
    /// Subnet limit reached.
    SubnetFull,
    /// The spending deadline passed.
    Expired,
    /// Management custody currently excludes growth.
    Occupied,
    /// More liquid cycles are needed.
    Underfunded,
    /// Eligible to create a spare.
    Due,
}

#[cfg(feature = "standby")]
/// Unsigned read of the active Root policy and its inventory totals.
pub struct ReadSpareCounts;
#[cfg(feature = "standby")]
impl canic::method::Contract for ReadSpareCounts {
    const NAME: &'static str = "spare_counts";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = ();
    type Output = Option<SpareCounts>;
}

/// A surplus principal and the former fleet authority when its origin is known.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StandbyImport {
    /// Exact canister principal.
    pub principal: String,
    /// Former fleet Root which must explicitly release this candidate.
    #[serde(default)]
    pub previous_root: Option<String>,
}

/// One immutable fleet reserve approval. Imports identify unassigned, externally released assets.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StandbyPolicy {
    /// Addressed Root (its component name in a fleet declaration).
    pub root: String,
    /// Desired claimable spare count.
    pub minimum_available: u8,
    /// Lifetime number of new canisters this approval may pay for.
    pub maximum_creations: u8,
    /// Absolute cutoff for new paid dispatch, in IC nanoseconds.
    pub expires_at_ns: u64,
    /// Least observed cycle balance of a surplus canister at admission.
    pub minimum_import_cycles: std::num::NonZeroU64,
    /// Exact surplus principals released to this fleet by their former owner.
    #[serde(default)]
    pub imports: Vec<StandbyImport>,
}

/// Default managed-child backstop for one Root on its subnet.
pub const DEFAULT_SUBNET_CHILD_LIMIT: u32 = 40_000;

/// Controller-approved child ceiling for this subnet owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetCapacity {
    /// Addressed management owner.
    pub root: String,
    /// Lifetime allocation limit across both pools.
    pub max_children: u32,
    /// Permit this exact approved limit to exceed the default subnet backstop.
    pub allow_large_subnet: bool,
}
impl SubnetCapacity {
    /// Validate both the count and the explicit opt-in above the ordinary backstop.
    #[must_use]
    pub const fn permits_limit(max_children: u32, allow_large_subnet: bool) -> bool {
        max_children > 0 && (max_children <= DEFAULT_SUBNET_CHILD_LIMIT || allow_large_subnet)
    }
}
/// Approve the fleet reserve target and paid creation allowance.
pub const CONFIGURE_STANDBY: u64 = ic_wire::method_id("configure_standby");

#[cfg(feature = "standby")]
/// Controller's exact reserve policy.
pub struct ConfigureStandby;
#[cfg(feature = "standby")]
impl canic::method::Contract for ConfigureStandby {
    const NAME: &'static str = "configure_standby";
    const ID: u64 = CONFIGURE_STANDBY;
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = StandbyPolicy;
    type Output = Result<(), canic::sharding::standby::ReserveError>;
}

#[cfg(feature = "standby")]
/// Shared controller-approved subnet ceiling.
pub struct ConfigureCapacity;
#[cfg(feature = "standby")]
impl canic::method::Contract for ConfigureCapacity {
    const NAME: &'static str = "configure_capacity";
    const ID: u64 = CONFIGURE_CAPACITY;
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = SubnetCapacity;
    type Output = Result<(), crate::Refusal>;
}

/// Creation allowances retained by Root, including the approved requester and artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ChildCreationGrant {
    /// Children serving a hub’s shared routing pool.
    Shared(ShardPool),
    /// Children created for a registry’s opaque requests.
    Dedicated(InstancePool),
    /// Children in a hub’s worker pool.
    Worker(InstancePool),
    /// One child of each eligible parent in the registry.
    PerParent(SingletonGrant),
}

/// Wire identity for approving Root creation effects.
pub const APPROVE_CHILD_CREATION: u64 = ic_wire::method_id("canic.root.approve_child_creation.v1");

/// Controller approval for child creation under one requester grant.
pub struct ApproveChildCreation;
#[cfg(feature = "standby")]
impl canic::method::Contract for ApproveChildCreation {
    const NAME: &'static str = "approve_child_creation";
    const ID: u64 = APPROVE_CHILD_CREATION;
    const REQUEST: ic_wire::Limits = LIMITS;
    const REPLY: ic_wire::Limits = crate::LIMITS;
    type Input = ChildCreationGrant;
    type Output = Result<(), crate::Refusal>;
}

/// Admit the shared ceiling before configuring larger pools.
pub const CONFIGURE_CAPACITY: u64 = ic_wire::method_id("canic.placement.configure_capacity.v1");

/// Count-only change bound to the complete previous grant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IncreasePool {
    /// Current shared-shard grant and its new count.
    Shards(ShardPool, u8),
    /// Current dedicated-instance grant and its new count.
    Instances(InstancePool, u8),
}
/// Increase one pool count without renewing funding or time.
pub const INCREASE_POOL: u64 = ic_wire::method_id("canic.placement.increase_pool.v1");

/// Supplier-owned Store frame budgets.
pub const STORE_LIMITS: ic_wire::Limits = {
    let limits = ic_wasm_store_protocol::LIMITS;
    ic_wire::Limits {
        bytes: limits.bytes,
        nodes: limits.nodes,
        depth: limits.depth,
        allocation: limits.allocation,
    }
};
/// Supplier chunk budget for the Canic Wire codec.
pub const STORE_CHUNK_LIMITS: ic_wire::Limits = {
    let limits = ic_wasm_store_protocol::CHUNK_LIMITS;
    ic_wire::Limits {
        bytes: limits.bytes,
        nodes: limits.nodes,
        depth: limits.depth,
        allocation: limits.allocation,
    }
};

/// Controller approval for destructive retirement under an immutable Store release.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChildRetirement {
    /// Root allowed to execute the retirement.
    pub root: String,
    /// Installed shard principal and original allocation.
    pub child: String,
    /// Original placement slot.
    pub slot: u32,
    /// Original placement generation.
    pub generation: [u8; 32],
    /// Required balance when returning the running empty principal.
    pub minimum_reuse_cycles: std::num::NonZeroU64,
    /// Absolute cutoff for new effects, including restart after the proof wait.
    pub expires_at_ns: u64,
}

/// Progress of one immutable retirement approval.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RetirementPhase {
    /// Exact release pin dispatched. Resume by observation.
    PinIssued,
    /// Root owns the pin.
    Pinned,
    /// Store cache preparation dispatched.
    CacheIssued,
    /// Root observed every approved chunk.
    Cached,
    /// Destructive dormant installation dispatched, never repeated.
    InstallIssued,
    /// Approved code and receipt observed; original proof deadline still applies.
    Dormant,
    /// Stop dispatched.
    StopIssued,
    /// Uninstall dispatched.
    UninstallIssued,
    /// Empty module observed.
    Uninstalled,
    /// Start dispatched.
    StartIssued,
    /// Returned once to the shared physical inventory.
    Returned,
}

/// Advance the exact controller-approved shard retirement.
#[cfg(feature = "retirement")]
pub struct RetireReleasedChild;
#[cfg(feature = "retirement")]
impl canic::method::Contract for RetireReleasedChild {
    const NAME: &'static str = "retire_released_child";
    const ID: u64 = ic_wire::method_id("canic.root.retire_released_child.v1");
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = Stage<ChildRetirement>;
    type Output = Result<RetirementPhase, canic::sharding::standby::ReserveError>;
}

/// A finite recurring debit approved for one directly installed service.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildFunding {
    /// Principal that owns the cycles.
    pub parent: ic_platform_types::Principal,
    /// Exact child installation and declared component.
    pub child: ic_platform_types::Principal,
    /// Name retained by Root when installing the service.
    pub component: String,
    /// Trigger only below this working balance.
    pub below_cycles: u64,
    /// One payment always transfers this amount in full.
    pub amount_cycles: u64,
    /// Total permitted debit for this child including unresolved payments.
    pub budget_cycles: u64,
    /// No new deposits after this IC timestamp.
    pub expires_at_ns: u64,
}

/// Refusal or unresolved outcome of an ongoing parent-to-child funding operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundingError {
    /// Caller lacks authority for this funding edge.
    Unauthorized,
    /// Child has no acknowledged funding edge.
    UnknownChild,
    /// The rule has invalid terms or a different parent.
    InvalidApproval,
    /// The installed funding rule differs.
    ApprovalConflict,
    /// The funding grant has expired.
    Expired,
    /// The funding ledger is full.
    Capacity,
    /// The approved parent has no ready installation.
    ParentUnavailable,
    /// The child installation changed.
    GenerationMismatch,
    /// The payment sequence does not follow the recorded receipt.
    InvalidSequence,
    /// The funding edge is closed.
    Closed,
    /// The original debit is still outstanding.
    PaymentPending,
    /// The reported balance is above the funding threshold.
    AboveThreshold,
    /// The grant cannot cover the fixed deposit.
    BudgetExhausted,
    /// The parent cannot cover the deposit.
    InsufficientCycles,
    /// The failed deposit was fully refunded.
    DepositRefunded,
    /// The forwarded request was refused before dispatch.
    CallNotSubmitted,
    /// The forwarded request has no verified reply.
    ReplyUnconfirmed,
    /// A retained management operation prevents this change.
    CustodyOccupied,
}

impl std::fmt::Display for FundingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for FundingError {}

/// Approve the rule once or observe the original grant.
#[cfg(feature = "standby")]
pub struct ConfigureChildFunding;
#[cfg(feature = "standby")]
impl canic::method::Contract for ConfigureChildFunding {
    const NAME: &'static str = "configure_child_funding";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = ChildFunding;
    type Output = Result<ChildFundingStatus, FundingError>;
}

/// The lifetime exposure and outcome of the latest payment.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChildFundingStatus {
    /// Includes the amount of any unresolved deposit.
    pub charged_cycles: u64,
    /// Number of deposits dispatched.
    pub attempts: u64,
    /// Last deposit is awaiting its original callback.
    pub issued: bool,
    /// Original refund on the latest completed deposit, if known.
    pub refund_cycles: Option<u128>,
    /// Revocation or failure prevents further spending.
    pub closed: bool,
}

/// Revoke future payments without erasing an outstanding debit.
#[cfg(feature = "standby")]
pub struct StopChildFunding;
#[cfg(feature = "standby")]
impl canic::method::Contract for StopChildFunding {
    const NAME: &'static str = "stop_child_funding";
    const ID: u64 = ic_wire::method_id("canic.owner.stop_child_funding.v1");
    type Input = ic_platform_types::Principal;
    type Output = Result<ChildFundingStatus, FundingError>;
}

/// Operator-approved settings for a fresh Root installation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RootBootstrap {
    /// Permanent children included in the initial funding calculation.
    pub services: u32,
    /// Maximum managed children for this Root.
    pub max_children: u32,
    /// Explicit operator exception to the standard subnet ceiling.
    pub allow_large_subnet: bool,
}

impl RootBootstrap {
    /// Check the bootstrap count and the exceptional subnet declaration.
    #[must_use]
    pub fn valid(self) -> bool {
        self.services > 0
            && self.services <= self.max_children
            && SubnetCapacity::permits_limit(self.max_children, self.allow_large_subnet)
    }
}
