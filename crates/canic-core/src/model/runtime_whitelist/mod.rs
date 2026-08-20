//! Module: model::runtime_whitelist
//!
//! Responsibility: own authoritative runtime-whitelist state and mutation identities.
//! Does not own: stable-memory access, endpoint authorization, or Candid dispatch.
//! Boundary: pure policy returns complete model records; ops converts them to storage and DTOs.

use crate::cdk::types::Principal;

/// Current product schema. Pre-1.0 changes hard-cut this value in place.
pub const RUNTIME_WHITELIST_SCHEMA_VERSION: u32 = 1;
/// Maximum canonical principals admitted by one managed Canister.
pub const MAX_RUNTIME_WHITELIST_PRINCIPALS: usize = 256;
/// Maximum principals returned by one status page.
pub const MAX_RUNTIME_WHITELIST_PAGE: u64 = 128;
/// Maximum encoded canonical record size admitted to memory ID 61.
pub const MAX_RUNTIME_WHITELIST_RECORD_BYTES: u32 = 32 * 1024;

/// Closed mutation action used by canonical operation hashing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeWhitelistAction {
    Add,
    Remove,
}

/// Model-owned semantic outcome of one accepted mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeWhitelistMutationOutcomeModel {
    Added,
    AlreadyPresent,
    Removed,
    AlreadyAbsent,
}

impl RuntimeWhitelistAction {
    #[must_use]
    pub const fn hash_byte(self) -> u8 {
        match self {
            Self::Add => 0,
            Self::Remove => 1,
        }
    }
}

/// One retained accepted operation, sufficient for exact retry only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWhitelistOperation {
    pub operation_id: [u8; 32],
    pub request_hash: [u8; 32],
    pub response: RuntimeWhitelistMutationResponseModel,
}

/// Model-owned exact mutation response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWhitelistMutationResponseModel {
    pub outcome: RuntimeWhitelistMutationOutcomeModel,
    pub principal: Principal,
    pub revision: u64,
    pub membership_digest: [u8; 32],
}

/// Sole canonical runtime-whitelist authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWhitelistState {
    pub schema_version: u32,
    pub principals: Vec<Principal>,
    pub revision: u64,
    pub membership_digest: [u8; 32],
    pub last_operation: Option<RuntimeWhitelistOperation>,
}
