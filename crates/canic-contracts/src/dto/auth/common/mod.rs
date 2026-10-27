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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum DelegationAudience {
    Fleet(FleetKey),
}

//
// DelegatedRoleGrant
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DelegatedRoleGrant {
    pub target: CanisterRole,
    pub scopes: Vec<String>,
}
