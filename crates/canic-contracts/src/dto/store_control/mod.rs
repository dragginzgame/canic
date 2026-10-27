//! Native boundary data for Store grants and cycle reclamation.
use serde::{Deserialize, Serialize};
/// Compare-and-set intent with the complete retained installation authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FixtureGrantRequest {
    pub expected_revision: u64,
    pub binding: crate::dto::runtime_reporting::FixtureTargetBinding,
    pub enabled: bool,
}
/// Observed transfer result; these fields do not confer new spending authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReclamationResponse {
    pub destination: ic_platform::Principal,
    pub cycles_before: u128,
    pub retained_cycles_target: u128,
    pub cycles_transferred: u128,
    pub cycles_after: u128,
}
