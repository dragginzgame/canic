//! Module: dto::auth::common
//!
//! Responsibility: shared delegated-auth DTO atoms.
//! Does not own: validation, canonicalization, or authorization policy.
//! Boundary: passive fields reused by auth proof, token, and renewal DTOs.

use crate::dto::prelude::*;
use crate::ids::FleetKey;

//
// DelegationAudience
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DelegationAudience {
    Fleet(FleetKey),
}

//
// DelegatedRoleGrant
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegatedRoleGrant {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}

//
// AuthRequestMetadata
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthRequestMetadata {
    pub request_id: [u8; 32],
    pub ttl_ns: u64,
}
