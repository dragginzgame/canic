//! Native pool retry boundary data; pool operation state remains in its original owner.
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Exact tracked canister selected for another reset attempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ResetRequest {
    pub canister_id: Principal,
}

/// Exact canister whose reset was queued.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ResetResponse {
    pub canister_id: Principal,
}

/// Original operation retained by the scheduled refill retry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RefillResponse {
    pub previous_operation_id: [u8; 32],
}
