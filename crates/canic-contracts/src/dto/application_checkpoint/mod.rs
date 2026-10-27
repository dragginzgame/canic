//! Module: dto::application_checkpoint
//!
//! Responsibility: carry opt-in application checkpoint participant requests and views.
//! Boundary: replies are passive; Root independently qualifies complete unit consistency.

use crate::ids::ApplicationCheckpointBinding;
use serde::{Deserialize, Serialize};

pub use crate::ids::ApplicationCheckpointPhase;

/// Prepare, observe or release one exact checkpoint at its installed participant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationCheckpointRequest {
    pub binding: ApplicationCheckpointBinding,
}

/// Reestablish a fresh checkpoint after loading its exact prepared source snapshot.
///
/// The authenticated Root must qualify source bytes, same-release same-ID loading
/// and outside-snapshot custody before invoking this participant transition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationCheckpointRestoreRequest {
    pub captured: ApplicationCheckpointBinding,
    pub binding: ApplicationCheckpointBinding,
}

/// Exact local record projection, without a global checkpoint or IC-effect claim.
///
/// Absence proves neither that an earlier message cannot arrive nor nonapplication.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationCheckpointStatusResponse {
    pub retained: Option<ApplicationCheckpointParticipantStatus>,
}

/// Installation-bound local acknowledgement retained by application-owned storage.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationCheckpointParticipantStatus {
    pub binding: ApplicationCheckpointBinding,
    pub phase: ApplicationCheckpointPhase,
}

/// Original Root request; expected sequence prevents delayed requests issuing a new unit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootCheckpointBeginRequest {
    pub root_install_id: [u8; 32],
    pub operation_id: [u8; 32],
    pub expected_sequence: u64,
}

/// Exact original checkpoint selection for advance and release.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootCheckpointRequest {
    pub root_install_id: [u8; 32],
    pub operation_id: [u8; 32],
    pub sequence: u64,
}

/// Compact retained Root progress; it is not snapshot or spending authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootCheckpointStatus {
    pub root_install_id: [u8; 32],
    pub operation_id: [u8; 32],
    pub sequence: u64,
    pub unit_digest: [u8; 32],
    pub phase: ApplicationCheckpointPhase,
    pub participants: u32,
    pub acknowledged: u32,
}

/// Wire hook request bound to the receiver's independently installed full authority.
///
/// The digest commits the canonical receiver/Root installations, Fleet and policy.
/// The receiver reconstructs its binding from local authority, never from host assertions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ApplicationCheckpointCommand {
    pub authority_digest: [u8; 32],
    pub operation_id: [u8; 32],
    pub sequence: u64,
    pub unit_digest: [u8; 32],
    pub phase: ApplicationCheckpointPhase,
}
