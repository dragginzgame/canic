//! Native reserve evidence; no funding authority or management effects.
use ic_platform::Principal;
use serde::{Deserialize, Serialize};
/// Indicative reserve bound to one Root and one management-call target.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct InspectionReserve {
    pub caller: Principal,
    pub canister_id: Principal,
    pub native_cycles: u128,
    pub available_liquid_cycles: u128,
    pub required_liquid_cycles: u128,
}
