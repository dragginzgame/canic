//! Native capacity import steps; durable reservation and spending authority stay canonical.
use crate::dto::pool_import::PoolImportIdentity;
use serde::{Deserialize, Serialize};
/// Advance only the exact reviewed source.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct AdvanceRequest {
    pub identity: PoolImportIdentity,
    pub canister_id: ic_platform::Principal,
}
/// Release only against the reviewed publication evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReleaseRequest {
    pub identity: PoolImportIdentity,
    pub publication_sha256: [u8; 32],
}
