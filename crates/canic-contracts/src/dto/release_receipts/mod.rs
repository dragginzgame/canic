//! Module: dto::release_receipts
//!
//! Responsibility: carry controller-only discovery of retained shared replay authority.
//! Does not own: authentication, replay decisions, accounting or settlement.
//! Boundary: metadata preserves original identities without returning cached application replies.

use candid::{CandidType, Principal};
use serde::Deserialize;

/// Authentication class retained by the original operation, independent of the current observer.

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum ReplayReleaseAuthentication {
    DirectCaller,
    DelegatedToken,
    RoleAttestation,
}

/// Exact retained replay phase; expiry never proves an uncertain paid effect absent.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum ReplayReleasePhase {
    Reserved,
    ExternalEffectInFlight,
    Committed,
    RecoveryRequired(ReplayReleaseRecoveryReason),
}

/// Original recovery reason, including unsettled accounting after a known effect.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum ReplayReleaseRecoveryReason {
    ExternalEffectStatusUnknown,
    ComponentChildLifecycleInterrupted,
    ResponseCommitFailed,
    CostSettlementFailed,
}

/// Original effect identity; no field authorizes another dispatch.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum ReplayReleaseEffect {
    RootCanisterProvision { command_kind: String },
    ManagementCall { canister: Principal, method: String },
    IcpTransfer { operation_id: [u8; 32] },
}

/// Retained accounting state; TTL expiry does not establish an external effect's outcome.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum ReplayReleaseIntentState {
    Pending,
    Committed,
    Aborted,
}

/// Exact bounded local reservation record, observed in the same query as its receipt.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ReplayReleaseIntent {
    pub resource_key: String,
    pub quantity: u64,
    pub state: ReplayReleaseIntentState,
    pub created_at_secs: u64,
    pub ttl_secs: Option<u64>,
}

/// Original cost-guard identities and current records; missing records are not settlement proof.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ReplayReleaseSettlement {
    pub quota_intent_id: u64,
    pub reservation_intent_id: u64,
    pub quota: Option<ReplayReleaseIntent>,
    pub reservation: Option<ReplayReleaseIntent>,
}

/// Compact authority metadata for one retained replay receipt.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ReplayReleaseEntry {
    pub slot: [u8; 32],
    pub command_kind: String,
    pub operation_id: [u8; 32],
    pub actor: Principal,
    pub authentication: ReplayReleaseAuthentication,
    pub payload_hash_schema_version: u32,
    pub payload_hash: [u8; 32],
    pub phase: ReplayReleasePhase,
    pub created_at_ns: u64,
    pub updated_at_ns: u64,
    pub expires_at_ns: Option<u64>,
    pub cost_guard_settlement: Option<ReplayReleaseSettlement>,
    pub effect: Option<ReplayReleaseEffect>,
}

/// One bounded stable row per page, with key-only lookahead and no expiry pruning.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ReplayReleaseResponse {
    pub owner: Principal,
    pub entry: Option<ReplayReleaseEntry>,
    pub next_after: Option<[u8; 32]>,
}
