//! Installed volatile Auth receiver fixture with direct Canic Root enrollment.
//! Fixture control is installation-owned, never browser-selected authority.
#[cfg(target_arch = "wasm32")]
mod canister;

pub use canic::auth::enrollment;
pub use canic::auth::protocol;
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
