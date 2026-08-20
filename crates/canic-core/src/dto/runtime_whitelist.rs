//! Passive boundary DTOs for managed-role runtime whitelist administration.

use crate::dto::{page::Page, prelude::*};

/// One bounded runtime-whitelist mutation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RuntimeWhitelistMutationRequest {
    pub principal: Principal,
    pub expected_revision: u64,
    pub operation_id: [u8; 32],
}

/// Exact mutation selected beneath the managed-role command envelope.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum RuntimeWhitelistCommand {
    Add(RuntimeWhitelistMutationRequest),
    Remove(RuntimeWhitelistMutationRequest),
}

/// Stable semantic outcome of one accepted mutation.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RuntimeWhitelistMutationOutcome {
    Added,
    AlreadyPresent,
    Removed,
    AlreadyAbsent,
}

/// Exact accepted mutation result, including idempotent outcomes.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeWhitelistMutationResponse {
    pub outcome: RuntimeWhitelistMutationOutcome,
    pub principal: Principal,
    pub revision: u64,
    pub membership_digest: [u8; 32],
}

/// Bounded canonical membership status.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RuntimeWhitelistStatusResponse {
    pub principals: Page<Principal>,
    pub revision: u64,
    pub membership_digest: [u8; 32],
    pub maximum_principals: u16,
}
