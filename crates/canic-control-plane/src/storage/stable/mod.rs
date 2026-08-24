#[cfg(feature = "root-control-plane")]
pub mod canister_pool;
#[cfg(feature = "root-control-plane")]
pub mod component_provisioning;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod component_registry;
pub mod fleet_admission;
pub mod fleet_coordinator;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod fleet_registry_mirror;
#[cfg(feature = "root-control-plane")]
pub mod root_admission;
#[cfg(feature = "root-control-plane")]
pub mod root_funding;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod state;
#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
pub mod template;
