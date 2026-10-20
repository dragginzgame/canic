//! Module: dto::application_checkpoint
//!
//! Responsibility: carry opt-in application checkpoint participant requests and views.
//! Boundary: replies are passive; Root independently qualifies complete unit consistency.

use crate::ids::ApplicationCheckpointBinding;
use ic_platform::candid::contract::CandidType;
use serde::{Deserialize, Serialize};

pub use crate::ids::ApplicationCheckpointPhase;

/// Prepare, observe or release one exact checkpoint at its installed participant.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
#[serde(deny_unknown_fields)]
pub struct ApplicationCheckpointRequest {
    pub binding: ApplicationCheckpointBinding,
}

/// Reestablish a fresh checkpoint after loading its exact prepared source snapshot.
///
/// The authenticated Root must qualify source bytes, same-release same-ID loading
/// and outside-snapshot custody before invoking this participant transition.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
#[serde(deny_unknown_fields)]
pub struct ApplicationCheckpointRestoreRequest {
    pub captured: ApplicationCheckpointBinding,
    pub binding: ApplicationCheckpointBinding,
}

/// Exact local record projection, without a global checkpoint or IC-effect claim.
///
/// Absence proves neither that an earlier message cannot arrive nor nonapplication.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct ApplicationCheckpointStatusResponse {
    pub retained: Option<ApplicationCheckpointParticipantStatus>,
}

/// Installation-bound local acknowledgement retained by application-owned storage.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct ApplicationCheckpointParticipantStatus {
    pub binding: ApplicationCheckpointBinding,
    pub phase: ApplicationCheckpointPhase,
}
