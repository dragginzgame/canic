//! Module: dto::auth::application_session
//!
//! Responsibility: define passive local application-session command and status contracts.
//! Does not own: proof verification, authority policy, persistence, or caller acquisition.
//! Boundary: cfg-pruned managed role variants carry these values over Candid.

use super::DelegatedToken;
use crate::{
    dto::{page::Page, prelude::*},
    ids::{CanisterRole, FleetKey},
};

/// Request to establish one caller-bound scoped application session.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationSessionRequest {
    pub delegated_token: DelegatedToken,
    pub requested_scopes: Vec<String>,
    pub requested_ttl_secs: Option<u64>,
}

/// Non-secret caller-self projection of one retained application session.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationSessionView {
    pub authenticated_subject: Principal,
    pub issuer: Principal,
    pub scopes: Vec<String>,
    pub established_at_ns: u64,
    pub expires_at_ns: u64,
    pub authority_generation: u64,
}

/// Exact inactive classification for a physically absent or invalid caller session.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InactiveApplicationSession {
    Missing,
    Expired {
        expired_at_ns: u64,
    },
    StaleFleet,
    StaleRole,
    StaleGeneration {
        session_generation: u64,
        current_generation: u64,
    },
    InadmissibleSubject,
}

/// Caller-self application-session status.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ApplicationSessionStatus {
    Active(ApplicationSessionView),
    Inactive(InactiveApplicationSession),
}

/// Non-secret verifier policy that selects the accepted root registry authority.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationSessionVerifierPolicyView {
    pub root_canister_id: Principal,
    pub minimum_accepted_registry_epoch: Option<u64>,
}

/// Protected configuration and current runtime binding for local authorization.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationSessionPolicyView {
    pub fleet: FleetKey,
    pub role: CanisterRole,
    pub authority_generation: u64,
    pub allowed_scopes: Vec<String>,
    pub default_session_ttl_secs: u64,
    pub maximum_session_ttl_secs: u64,
    pub proof_lifetime_ceiling_ns: u64,
    pub verifier: ApplicationSessionVerifierPolicyView,
}

/// One bounded operator-only session row, including inactive retained records.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApplicationSessionAuditEntry {
    pub transport_caller: Principal,
    pub status: ApplicationSessionStatus,
}

/// Protected operator inspection of declared policy, runtime binding and retained sessions.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct ApplicationSessionAuditResponse {
    pub policy: ApplicationSessionPolicyView,
    pub sessions: Page<ApplicationSessionAuditEntry>,
}

/// Managed role command nested under the existing `canic_command` method.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the bounded Candid command keeps its accepted direct request shape"
)]
pub enum ApplicationSessionCommand {
    Establish(ApplicationSessionRequest),
    Clear,
}

/// Managed role response nested under the existing `canic_command` method.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ApplicationSessionCommandResponse {
    Established(ApplicationSessionView),
    Cleared,
}
