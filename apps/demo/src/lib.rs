//! Application-owned counter contract. Persistence and endpoints compile only for Wasm.
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
mod actor;

/// Wire identity of the replicated `counter` endpoint.
pub const METHOD: u64 = 1;
/// Fixed request, response and logical-record budget.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 256,
    allocation: 4096,
    nodes: 64,
    depth: 8,
};

/// Explicit application operations. Caller identity always comes from ingress.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Request {
    /// Observe the current value and fence without changing them.
    Read,
    /// Increment once; no automatic retry or application deduplication is promised.
    Increment,
    /// Close writes under a new monotonic nonzero maintenance attempt.
    Fence {
        /// Application maintenance identity, selected by the installation authority.
        attempt: u64,
    },
    /// Explicitly reopen the matching fence. A stale attempt cannot activate a newer one.
    Activate {
        /// Exact previously admitted attempt, never a caller assertion of authority.
        attempt: u64,
    },
}

/// Bounded public application observation; no authority is granted by this report.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snapshot {
    /// Number of committed increments.
    pub value: u64,
    /// Current closed-admission attempt, if any.
    pub fence: Option<u64>,
    /// Executable marker for the qualified A/B upgrade, not a storage schema.
    pub executable: u8,
}

/// Refusals happen before mutation. Corrupt storage or post-write failures trap.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Error {
    /// Maintenance is restricted to the actual original installation caller.
    NotOwner,
    /// Increment is closed by the current fence.
    Fenced,
    /// The counter cannot grow beyond u64::MAX.
    Overflow,
    /// Attempt is zero, already spent, stale or conflicts with the current fence.
    Attempt,
}
