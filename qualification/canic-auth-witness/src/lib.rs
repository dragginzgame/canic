//! Installed volatile Auth receiver fixture with direct Canic Root enrollment.
//! Fixture control is installation-owned, never browser-selected authority.
#[cfg(not(feature = "stable"))]
mod canister;
mod frames;
mod issuer;
#[cfg(all(feature = "profile", not(feature = "stable")))]
mod profile;
#[cfg(feature = "stable")]
mod stable;

pub use canic::auth::{enrollment, protocol};
use ic_auth_protocol_types::{AudienceId, CanonicalId};
use serde::{Deserialize, Serialize};

/// A fixed test namespace, independent of credential-supplied audience fields.
#[must_use]
pub const fn audience() -> AudienceId {
    AudienceId {
        canonical_network_id: CanonicalId::from_bytes([0x11; 32]),
        fleet_id: CanonicalId::from_bytes([0x22; 32]),
    }
}
/// Public Ed25519 test verification key (private seed is `[7; 32]`).
pub const KEY: [u8; 32] = [
    234, 74, 108, 99, 226, 156, 82, 10, 190, 245, 80, 123, 19, 46, 197, 249, 149, 71, 118, 174,
    190, 190, 123, 146, 66, 30, 234, 105, 20, 70, 210, 44,
];
/// Test observation contract.
pub const OBSERVE: u64 = 901;
/// Application endpoint budget for the witness's retained issuer callback.
pub const CALLBACK_LIMITS: ic_wire::Limits = {
    let limits = canic::auth::issuer::REQUEST_LIMITS;
    ic_wire::Limits {
        bytes: limits.bytes,
        allocation: limits.allocation,
        nodes: limits.nodes,
        depth: limits.depth,
    }
};
/// Small fixture control/observation budget.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 16384,
    nodes: 128,
    depth: 8,
};

/// Passive counts; never a readiness or authority capability.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// Whether the original volatile receiver still exists.
    pub present: bool,
    /// Mutating handlers entered, including ordinary refusals.
    pub calls: u32,
    /// Application writes performed under current Auth authorization.
    pub writes: u32,
    /// Sessions, replay records and declared encoded bytes for the fixed account.
    pub usage: (u64, u64, u64),
    /// Retained proof-verification work credit.
    pub credit: u32,
    /// Supplier metadata revision/generation, independent of host entry projection.
    pub enrollment: Option<(u64, u64)>,
}

/// Independent fixture counters at actual enrollment commit boundaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshObservation {
    /// Successful local reservations, before the entropy await.
    pub reserved: u32,
    /// Prepared direct Root calls, before the snapshot await.
    pub dispatched: u32,
    /// Authenticated Root replies captured before an optional fixture-only barrier.
    pub fetched: u32,
    /// Committed supplier metadata and application entry publications.
    pub installed: u32,
    /// Original receipt deadline of the latest successful reservation.
    pub deadline_ns: u64,
}

/// Issuer fixture observations independent of returned credentials.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuerObservation {
    /// Original volatile owner still exists.
    pub present: bool,
    /// Completed local signatures committed with the corresponding result.
    pub signatures: u32,
    /// Supplier retained operation count.
    pub operations: u32,
    /// Supplier retained login count.
    pub logins: u32,
    /// Supplier current admission epoch.
    pub epoch: u64,
}
