//! Hub-facing placement contract. The application derives one opaque key per account.

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
mod client;

/// Optional hub and shard composition for an issuer-qualified account.
#[cfg(feature = "method")]
pub mod provisioning;

#[cfg(target_arch = "wasm32")]
pub use client::{assign, lookup};
use serde::{Deserialize, Serialize};

/// Assign a key to an admitted shard or reserve its first allocation.
pub const ASSIGN: u64 = 91;
/// Observe the original allocation without reserving capacity.
pub const LOOKUP: u64 = 92;
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
