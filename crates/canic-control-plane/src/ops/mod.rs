//! Control-plane ops helpers over local state and template storage.

#[cfg(feature = "fleet-coordinator-canister")]
pub mod fleet_coordinator;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod storage;
