//! Public control-plane APIs for lifecycle, state, and template publication.

#[cfg(feature = "root-control-plane")]
pub mod component_auth;
#[cfg(feature = "fleet-coordinator-canister")]
pub mod fleet_coordinator;
#[cfg(feature = "root-control-plane")]
pub mod lifecycle;
#[cfg(feature = "root-control-plane")]
pub mod state;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod template;
