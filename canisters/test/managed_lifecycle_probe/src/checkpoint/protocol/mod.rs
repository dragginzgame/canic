//! Native boundary data for the installed checkpoint fixture.

use serde::{Deserialize, Serialize};

/// Complete authorities require cumulative allocation and traversal admission.
/// The two-binding restore request retains the scalar probe's byte/depth ceilings.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    allocation: 2_097_152,
    nodes: 4096,
    ..crate::protocol::REQUEST_LIMITS
};

/// The checkpoint view returns the same complete authority under its existing byte cap.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    allocation: REQUEST_LIMITS.allocation,
    ..crate::protocol::RESPONSE_LIMITS
};

/// Complete native checkpoint binding for the fixture's transition boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Binding {
    pub authority: canic::dto::caller_authority_status::CallerReceiverAuthority,
    pub operation_id: [u8; 32],
    pub sequence: u64,
    pub unit_digest: [u8; 32],
}

/// Explicit transition request, independent of its application-owned storage record.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TransitionRequest {
    pub binding: Binding,
    pub phase: canic::api::application_checkpoint::ApplicationCheckpointPhase,
}

/// Exact captured source and fresh checkpoint authority for same-release restore.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RestoreRequest {
    pub captured: Binding,
    pub binding: Binding,
}

/// Retained checkpoint projection, without changing the application's stored encoding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ParticipantStatus {
    pub binding: Binding,
    pub phase: canic::api::application_checkpoint::ApplicationCheckpointPhase,
}

pub const TRANSITION: u64 = 0x5445_5354_0000_0406;
pub const RESTORE: u64 = 0x5445_5354_0000_0407;
pub const INCREMENT: u64 = 0x5445_5354_0000_0408;
pub const VIEW: u64 = 0x5445_5354_0000_0409;
