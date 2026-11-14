//! Bounded application contracts for the standalone profile demo.
use serde::{Deserialize, Serialize};

pub mod growth;

#[cfg(test)]
mod tests;

/// Update a caller-owned profile with optimistic revision checking.
pub const PUT: u64 = 1;
/// Read the caller's private profile.
pub const GET: u64 = 2;
/// Read public application readiness and fixed budgets.
pub const INFO: u64 = 3;
/// Read the entry's immutable worker route (query or replicated resolution).
pub const ROUTE: u64 = 4;
/// Bind an entry to one worker, once, under controller authority.
pub const BIND: u64 = 5;
/// Ask the entry to check its worker using one bounded direct call.
pub const CHECK: u64 = 6;
/// Maximum admitted profiles in this demo.
pub const MAX_PROFILES: u64 = 16;
/// Maximum UTF-8 display-name bytes.
pub const MAX_NAME_BYTES: usize = 128;
/// Complete Wire frame and decoding ceilings.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 16384,
    nodes: 256,
    depth: 8,
};

/// One idempotent revision transition. New registrations use revision zero.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Put {
    /// Application-supplied nonzero identity, immutable after registration.
    pub user_id: u64,
    /// Current revision, or zero for a new profile.
    pub expected_revision: u64,
    /// Nonempty display name, at most 128 UTF-8 bytes.
    pub display_name: String,
}

/// Caller-owned private application state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Profile {
    /// Immutable application identity.
    pub user_id: u64,
    /// Original authenticated IC caller's principal bytes.
    pub owner: Vec<u8>,
    /// Monotonically increasing application revision, starting at one.
    pub revision: u64,
    /// Private display name.
    pub display_name: String,
}

/// Typed refusals; none authorizes changing caller identity or retrying new work.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProfileError {
    /// Anonymous callers cannot use private operations.
    Unauthenticated,
    /// The target user ID belongs to another caller.
    Forbidden,
    /// A caller's immutable user mapping already names a different ID.
    AlreadyRegistered,
    /// No profile exists for the requested identity.
    NotFound,
    /// The request's name or identity is outside application bounds.
    InvalidInput,
    /// The current revision does not admit this transition or its exact replay.
    RevisionConflict,
    /// The configured profile limit is occupied.
    Capacity,
}

/// Result of an authenticated application operation.
pub type ProfileResult = Result<Profile, ProfileError>;

/// Public readiness only; it does not expose private profile contents or activity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Info {
    /// Observable executable identity for the preservation rehearsal.
    pub executable: String,
    /// True only after storage admission and strict application-store opening.
    pub ready: bool,
    /// Fixed application record limit.
    pub profile_limit: u64,
    /// Physical stable-memory ceiling in 64 KiB pages, including Storage controls.
    pub stable_page_limit: u64,
}

/// An application-owned route, not management or impersonation authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Route {
    /// Canonical text principal of the worker. Clients keep their own identity.
    pub worker: String,
}

/// Entry routing and observation failures; none permits silently choosing another worker.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RouteError {
    /// The controller has not bound this entry yet.
    Unbound,
    /// Binding and active checks require a current controller.
    Forbidden,
    /// An existing binding cannot be replaced.
    Conflict,
    /// Not a valid worker principal, or the entry itself.
    InvalidWorker,
    /// The direct call did not return an admitted response; worker health is unknown.
    Unavailable,
    /// Returned bytes did not satisfy the worker protocol.
    InvalidReply,
    /// One active check already occupies this entry's observation slot.
    Busy,
}

/// Immutable binding or explicit refusal.
pub type RouteResult = Result<Route, RouteError>;

/// A point-in-time worker response obtained by the entry through a direct call.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkerCheck {
    /// Exact bound route checked by this call.
    pub route: Route,
    /// Worker-reported readiness; this is not a management or module attestation.
    pub info: Info,
}

/// Direct observation or explicit unknown result.
pub type CheckResult = Result<WorkerCheck, RouteError>;
