//! Hub-facing placement contract. The application derives one opaque key per account.

#[cfg(test)]
mod tests;

mod client;
pub mod payment;
#[cfg(feature = "storage")]
pub mod registry;
#[cfg(feature = "storage")]
pub mod routes;
pub mod standby;

/// Refusal or unresolved outcome of shard inspection and drain
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum ShardError {
    /// Shard operation requires its authorized caller.
    Unauthorized,
    /// Unknown shard or pool.
    UnknownShard,
    /// Shard allocation changed.
    GenerationChanged,
    /// Invalid policy or incompatible with an already configured pool.
    InvalidPolicy,
    /// Root has not confirmed the child release.
    ReleaseUnconfirmed,
}

impl std::fmt::Display for ShardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unauthorized => "Shard operation requires its authorized caller",
            Self::UnknownShard => "Unknown shard or pool",
            Self::GenerationChanged => "Shard allocation changed",
            Self::InvalidPolicy => "Invalid or conflicting hub routing policy",
            Self::ReleaseUnconfirmed => "Root has not confirmed the child release",
        })
    }
}

impl std::error::Error for ShardError {}

/// Optional hub and shard composition for an issuer-qualified account.
pub mod provisioning;

pub use client::{ACQUISITION_CYCLES, Lookup, LookupPlacement, grow_shard};
use serde::{Deserialize, Serialize};

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
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub struct Allocation {
    /// Permanent child slot.
    pub slot: u32,
    /// Exact original generation, bound to Root, slot and approved Store creation.
    pub generation: [u8; 32],
}

/// Exact original destination of a placement under the addressed Root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub struct ShardTarget {
    /// Original slot and creation generation.
    pub allocation: Allocation,
    /// Installed canister principal. Worker release and reset use this field for the worker.
    pub shard: ic_platform_types::Principal,
}

/// Operator observation of one installed shard allocation.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub struct ShardStatus {
    /// Target to bind into a subsequent drain decision.
    pub target: ShardTarget,
    /// Whether the allocation has stopped accepting new placements.
    pub draining: bool,
    /// Live placements reported by the pool's route owner, including reservations.
    pub live_placements: u32,
}

/// Largest supported live occupancy of an individual shard.
pub const MAX_KEYS_PER_SHARD: u32 = 100_000;

/// Install an immutable routing policy at the owning hub.
pub struct ConfigureShardRoutes;
impl crate::method::Contract for ConfigureShardRoutes {
    const NAME: &'static str = "configure_shard_routes";
    const ID: u64 = ic_wire::method_id("canic.hub.configure_shard_routes.v1");
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = (String, u32);
    type Output = Result<(), ShardError>;
}

/// Observe a shard by principal in the addressed hub.
pub struct ObserveShard;

impl crate::method::Contract for ObserveShard {
    const NAME: &'static str = "observe_shard";
    const ID: u64 = ic_wire::method_id("canic.hub.observe_shard.v1");
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = ic_platform_types::Principal;
    type Output = Result<ShardStatus, ShardError>;
}

/// Approve drain for an exact installed shard allocation.
pub struct BeginShardDrain;

impl crate::method::Contract for BeginShardDrain {
    const NAME: &'static str = "begin_shard_drain";
    const ID: u64 = ic_wire::method_id("canic.hub.begin_shard_drain.v1");
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = ShardTarget;
    type Output = Result<u32, ShardError>;
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

/// Client interpretation of a Root placement reply.
#[derive(Debug, PartialEq, Eq)]
pub enum PlacementOutcome {
    /// No assignment has been reserved.
    Unknown,
    /// The original allocation is in progress.
    Pending(Allocation),
    /// Original allocation and its validated canister principal.
    Assigned(Allocation, ic_platform_types::Principal),
    /// The original allocation is unavailable, with its retained observation.
    Unavailable(Allocation, Option<String>),
    /// The pool is at capacity.
    Full,
    /// An earlier creation is outstanding.
    Busy,
    /// New creation lacks funding or approval.
    GrowthRefused,
}

fn assigned_principal(text: &str) -> Result<ic_platform_types::Principal, &'static str> {
    let principal =
        ic_platform_types::Principal::from_text(text).map_err(|_| "invalid assigned canister")?;
    if principal.to_text() != text || principal.as_slice().last() != Some(&1) {
        return Err("invalid assigned canister");
    }

    Ok(principal)
}

impl Placement {
    /// Consume a reply, keeping each placement state and its retained evidence.
    /// # Errors
    /// Refuses an assigned target that is not canonical canister text.
    pub fn into_outcome(self) -> Result<PlacementOutcome, &'static str> {
        Ok(match self {
            Self::Assigned(allocation, text) => {
                PlacementOutcome::Assigned(allocation, assigned_principal(&text)?)
            }
            Self::Pending(allocation) => PlacementOutcome::Pending(allocation),
            Self::Unavailable(allocation, reason) => {
                PlacementOutcome::Unavailable(allocation, reason)
            }
            Self::Unknown => PlacementOutcome::Unknown,
            Self::Full => PlacementOutcome::Full,
            Self::Busy => PlacementOutcome::Busy,
            Self::GrowthRefused => PlacementOutcome::GrowthRefused,
        })
    }

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
        Ok(Some((allocation, assigned_principal(text)?)))
    }
}

/// A parent reports that it has released this exact child generation.
pub struct ReleaseChildAllocation;
impl crate::method::Contract for ReleaseChildAllocation {
    const NAME: &'static str = "release_child_allocation";
    const ID: u64 = ic_wire::method_id("canic.root.release_child_allocation.v1");
    const REQUEST: ic_wire::Limits = LIMITS;
    type Input = (ic_platform_types::Principal, Allocation);
    type Output = Result<(), ReleaseError>;
}

/// Refusal to record a parent's release.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
pub enum ReleaseError {
    /// Child or generation differs from Root's retained creation.
    UnknownAllocation,
    /// Only the approved parent may release the child.
    Unauthorized,
}
