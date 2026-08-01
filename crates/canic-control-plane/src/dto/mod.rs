//! DTOs for control-plane Registry and template publication surfaces.

#[cfg(feature = "fleet-coordinator-canister")]
pub mod fleet_coordinator;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod template;
