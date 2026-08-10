//! Control-plane ops helpers over local state and template storage.

#[cfg(feature = "root-control-plane")]
pub mod canister_pool;
#[cfg(feature = "root-control-plane")]
pub mod component_directory_synchronization;
#[cfg(feature = "root-control-plane")]
pub mod component_provisioning;
#[cfg(feature = "root-control-plane")]
pub mod component_registry;
#[cfg(feature = "fleet-coordinator-canister")]
pub mod fleet_coordinator;
#[cfg(feature = "root-control-plane")]
pub mod fleet_registry_mirror;
#[cfg(feature = "root-control-plane")]
pub mod fleet_service_peer;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod storage;
