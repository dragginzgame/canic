//! Control-plane workflows for bootstrap and publication runtime.

#[cfg(feature = "root-control-plane")]
pub mod bootstrap;
#[cfg(feature = "root-control-plane")]
pub mod canister_pool;
#[cfg(feature = "root-control-plane")]
pub mod component_auth;
#[cfg(feature = "root-control-plane")]
pub mod component_directory_synchronization;
#[cfg(feature = "root-control-plane")]
pub mod component_provisioning;
#[cfg(feature = "root-control-plane")]
pub mod component_registry;
#[cfg(feature = "root-control-plane")]
pub mod component_rpc;
#[cfg(feature = "root-control-plane")]
pub mod deployment;
#[cfg(feature = "fleet-coordinator-canister")]
pub mod fleet_coordinator;
#[cfg(feature = "root-control-plane")]
mod fleet_coordinator_client;
#[cfg(feature = "root-control-plane")]
pub mod fleet_registry_mirror;
#[cfg(feature = "root-control-plane")]
pub mod fleet_subnet_root;
#[cfg(feature = "root-control-plane")]
pub mod root_admission;
#[cfg(feature = "root-control-plane")]
mod root_authority;
#[cfg(feature = "root-control-plane")]
pub mod root_funding;
#[cfg(feature = "root-control-plane")]
pub mod root_status;
#[cfg(feature = "root-control-plane")]
pub mod runtime;
#[cfg(feature = "root-control-plane")]
pub mod state;
#[cfg(feature = "wasm-store-canister")]
pub mod wasm_store;
