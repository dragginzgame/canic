//! Hub-facing placement contract. The application derives one opaque key per account.

#[cfg(test)]
mod tests;

mod client;

/// Optional hub and shard composition for an issuer-qualified account.
#[cfg(feature = "method")]
pub mod provisioning;

pub use client::{assign, assign_instance, lookup, lookup_instance};
use serde::{Deserialize, Serialize};

/// Assign a key to an admitted shard or reserve its first allocation.
pub const ASSIGN: u64 = 91;
/// Observe the original allocation without reserving capacity.
pub const LOOKUP: u64 = 92;
/// Reserve a child exclusively for one project key.
pub const ASSIGN_INSTANCE: u64 = ic_wire::method_id("canic.assign_instance.v1");
/// Observe a project key at the originally selected Root.
pub const LOOKUP_INSTANCE: u64 = ic_wire::method_id("canic.lookup_instance.v1");
/// An application-owned stable key, independent of the current login session.
pub type Key = [u8; 32];
/// Small request and reply frames for the hub.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 32768,
    nodes: 1024,
    depth: 12,
};

/// Identity scoped to the addressed Root and its immutable pool approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allocation {
    /// Permanent child slot.
    pub slot: u32,
    /// Exact original generation, bound to Root, slot and approved Store creation.
    pub generation: [u8; 32],
}

/// Exact original destination of a placement under the addressed Root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardTarget {
    /// Original slot and creation generation.
    pub allocation: Allocation,
    /// Installed shard principal.
    pub shard: ic_platform_types::Principal,
}

/// Operator observation of one installed shard allocation.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardStatus {
    /// Target to bind into a subsequent drain decision.
    pub target: ShardTarget,
    /// Whether the allocation has stopped accepting new placements.
    pub draining: bool,
    /// Current count of placement keys in Root's ledger.
    pub live_placements: u32,
}

/// Observe a shard by principal under the selected Root.
#[cfg(feature = "method")]
pub struct InspectShard;

#[cfg(feature = "method")]
impl crate::method::Contract for InspectShard {
    const NAME: &'static str = "inspect_shard";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = ic_platform_types::Principal;
    type Output = Result<ShardStatus, String>;
}

/// Approve drain for an exact installed shard allocation.
#[cfg(feature = "method")]
pub struct BeginShardDrain;

#[cfg(feature = "method")]
impl crate::method::Contract for BeginShardDrain {
    const NAME: &'static str = "begin_shard_drain";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = ShardTarget;
    type Output = Result<u32, String>;
}

/// Release one route after the hub has durably retired its application binding.
#[cfg(feature = "method")]
pub struct ReleaseShard;

#[cfg(feature = "method")]
impl crate::method::Contract for ReleaseShard {
    const NAME: &'static str = "release_shard";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = (Key, ShardTarget);
    type Output = Result<(), String>;
}

/// Lookup and assignment use the same explicit outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Placement {
    /// No assignment has been reserved.
    Unknown,
    /// A claim already consumes capacity while its exact creation is in progress.
    Pending(Allocation),
    /// Root retained the principal and observed code, controllers and readiness.
    Assigned(Allocation, String),
    /// The original allocation cannot currently serve this claim.
    Unavailable(Allocation, Option<String>),
    /// All approved shard capacity is reserved.
    Full,
    /// No claim was admitted while an earlier creation is outstanding.
    Busy,
    /// New creation lacks current funding or approval.
    GrowthRefused,
}

impl Placement {
    /// Return the retained allocation identity and its canonical canister target.
    /// Other placement states have no usable target; applications retain their original state.
    /// # Errors
    /// An invalid assigned principal is an unreadable result. Observe the original key.
    pub fn assigned_target(
        &self,
    ) -> Result<Option<(&Allocation, ic_platform_types::Principal)>, &'static str> {
        let Self::Assigned(allocation, text) = self else {
            return Ok(None);
        };
        let principal = ic_platform_types::Principal::from_text(text)
            .map_err(|_| "invalid assigned canister")?;
        if principal.to_text() != *text || principal.as_slice().last() != Some(&1) {
            return Err("invalid assigned canister");
        }
        Ok(Some((allocation, principal)))
    }
}
