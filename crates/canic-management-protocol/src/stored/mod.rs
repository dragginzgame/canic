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
/// Bounded metadata for two children and their eight retained upgrades each.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 4_194_304,
    nodes: 262_144,
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
    pub sources: [Option<Sources>; 2],
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
    /// Maximum lifetime keys per child.
    pub keys_per_shard: u16,
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
/// Maximum managed children across both pools of the single subnet owner.
pub const MAX_SUBNET_CHILDREN: u8 = 8;
/// Dedicated instances also consume the shared subnet allowance.
pub const MAX_INSTANCES: u8 = MAX_SUBNET_CHILDREN;

/// Controller's immutable child ceiling for this subnet owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetCapacity {
    /// Addressed management owner.
    pub root: String,
    /// Lifetime allocation limit across both pools.
    pub max_children: u8,
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

/// Maximum retained account keys per shard in the initial bounded Root layout.
pub const MAX_KEYS_PER_SHARD: u16 = 64;

/// Supplier-owned Store chunk budget for the Canic codec.
pub const STORE_CHUNK_LIMITS: ic_wire::Limits = {
    let limits = ic_wasm_store_protocol::CHUNK_LIMITS;
    ic_wire::Limits {
        bytes: limits.bytes,
        allocation: limits.allocation,
        nodes: limits.nodes,
        depth: limits.depth,
    }
};

/// Supplier-owned Store metadata budget for the Canic codec.
pub const STORE_LIMITS: ic_wire::Limits = {
    let limits = ic_wasm_store_protocol::LIMITS;
    ic_wire::Limits {
        bytes: limits.bytes,
        allocation: limits.allocation,
        nodes: limits.nodes,
        depth: limits.depth,
    }
};
