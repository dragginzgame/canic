//! Module: dto::auth::common
//!
//! Responsibility: shared delegated-auth DTO atoms.
//! Does not own: validation, canonicalization, or authorization policy.
//! Boundary: passive fields reused by auth proof, token, and renewal DTOs.

use crate::{dto::prelude::*, ids::FleetKey};

pub use ic_auth_protocol_types::AuthRequestMetadata;

//
// DelegationAudience
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum DelegationAudience {
    Fleet(FleetKey),
}

//
// DelegatedRoleGrant
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct DelegatedRoleGrant {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}
