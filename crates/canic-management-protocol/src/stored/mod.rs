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
    pub sources: [Option<Sources>; crate::children::ChildId::ALL.len()],
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
    /// Maximum live keys per child.
    pub keys_per_shard: u32,
    /// Lifetime ceiling on creation attachments; ordinary management fees are separate.
    pub creation_budget: u64,
    /// Last time a new child management effect may be dispatched.
    pub expires_at_ns: u64,
    /// Approved release and supplier.
    pub reference: Reference,
}
/// Controller approval frame for a fresh shard pool.
pub const CONFIGURE_SHARDS: u64 = 90;

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
    /// Lifetime ceiling for creation attachments.
    pub creation_budget: u64,
    /// Deadline for issuing new work.
    pub expires_at_ns: u64,
}

/// Approve a dedicated instance pool independently of the shared shard grant.
pub const CONFIGURE_INSTANCES: u64 = 93;
/// Approve the bounded worker allowance for one other hub.
pub const CONFIGURE_WORKERS: u64 = ic_wire::method_id("canic.configure_workers.v1");
/// Maximum managed children across all pools of the single subnet owner.
pub const MAX_SUBNET_CHILDREN: u8 = 8;
/// Dedicated instances also consume the shared subnet allowance.
pub const MAX_INSTANCES: u8 = MAX_SUBNET_CHILDREN;

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

/// Controller's immutable child ceiling for this subnet owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetCapacity {
    /// Addressed management owner.
    pub root: String,
    /// Lifetime allocation limit across both pools.
    pub max_children: u8,
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
    type Output = Result<(), String>;
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

/// Admit the shared ceiling before configuring larger pools.
pub const CONFIGURE_CAPACITY: u64 = ic_wire::method_id("canic.configure_capacity.v1");

/// Count-only change bound to the complete previous grant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IncreasePool {
    /// Current shared-shard grant and its new count.
    Shards(ShardPool, u8),
    /// Current dedicated-instance grant and its new count.
    Instances(InstancePool, u8),
}
/// Increase one pool count without renewing funding or time.
pub const INCREASE_POOL: u64 = ic_wire::method_id("canic.increase_pool.v1");

/// Absolute backstop for any single shared shard. Fleets approve their own lower limit.
pub const MAX_KEYS_PER_SHARD: u32 = 100_000;

/// Supplier-owned Store frame budgets.
pub use ic_wasm_store_protocol::{CHUNK_LIMITS as STORE_CHUNK_LIMITS, LIMITS as STORE_LIMITS};

/// Controller approval for destructive retirement under an immutable Store release.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShardRetirement {
    /// Root allowed to execute the retirement.
    pub root: String,
    /// Installed shard principal and original allocation.
    pub shard: String,
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
pub struct RetireShard;
#[cfg(feature = "retirement")]
impl canic::method::Contract for RetireShard {
    const NAME: &'static str = "retire_shard";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = Stage<ShardRetirement>;
    type Output = Result<RetirementPhase, String>;
}
