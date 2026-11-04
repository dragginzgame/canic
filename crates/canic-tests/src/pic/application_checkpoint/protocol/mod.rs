//! Expected native checkpoint fixture contracts for installed acceptance.

use serde::{Deserialize, Serialize};

/// Complete native checkpoint binding for the fixture's transition boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Binding {
    pub authority: canic::dto::caller_authority_status::CallerReceiverAuthority,
    pub operation_id: [u8; 32],
    pub sequence: u64,
    pub unit_digest: [u8; 32],
}

/// Explicit transition request, independent of its application-owned storage record.
#[derive(Clone, Deserialize, Serialize)]
pub struct TransitionRequest {
    pub binding: Binding,
    pub phase: canic::api::application_checkpoint::ApplicationCheckpointPhase,
}

/// Exact captured source and fresh checkpoint authority for same-release restore.
#[derive(Clone, Deserialize, Serialize)]
pub struct RestoreRequest {
    pub captured: Binding,
    pub binding: Binding,
}

/// Retained checkpoint projection, without changing the application's stored encoding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParticipantStatus {
    pub binding: Binding,
    pub phase: canic::api::application_checkpoint::ApplicationCheckpointPhase,
}
impl From<canic::ids::ApplicationCheckpointBinding> for Binding {
    fn from(value: canic::ids::ApplicationCheckpointBinding) -> Self {
        Self {
            authority: value.authority.into(),
            operation_id: value.operation_id,
            sequence: value.sequence,
            unit_digest: value.unit_digest,
        }
    }
}
