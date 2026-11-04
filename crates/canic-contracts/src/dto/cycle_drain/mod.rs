//! Exact application-owned retirement transfer; endpoint authority and custody stay with the source.

use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

/// Retained original operation and exact controlled destination/amount.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DrainRequest {
    pub operation_id: String,
    pub destination: Principal,
    pub cycles: u128,
}

/// Issuance evidence only; Host separately proves both controlled balance changes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum DrainResponse {
    Accepted { transferred_cycles: u128 },
    Replayed { transferred_cycles: u128 },
}
