//! Native binding of opaque application bytes to their allocated canister.
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Exact application bytes; their internal encoding remains application-owned.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentApplicationInitialization {
    pub target_canister: Principal,
    #[serde(with = "serde_bytes")]
    pub arguments: Vec<u8>,
}

/// Bind application initialization to the existing allocation operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootComponentInitializationRequest {
    pub operation_id: [u8; 32],
    pub initialization: ComponentApplicationInitialization,
}
