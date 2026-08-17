//! DTOs for control-plane Registry and template publication surfaces.

#[cfg(any(feature = "fleet-coordinator-canister", feature = "root-control-plane"))]
pub mod fleet_coordinator;
#[cfg(any(feature = "fleet-coordinator-canister", feature = "root-control-plane"))]
pub mod root;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod template;
