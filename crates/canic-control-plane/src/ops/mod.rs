//! Control-plane ops helpers over local state and template storage.

#[cfg(feature = "root-control-plane")]
pub mod component_registry;
#[cfg(feature = "fleet-coordinator-canister")]
pub mod fleet_coordinator;
#[cfg(feature = "root-control-plane")]
pub mod fleet_registry_mirror;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod storage;
