//! Control-plane workflows for bootstrap, publication runtime, and state queries.

#[cfg(feature = "root-control-plane")]
pub mod bootstrap;
#[cfg(feature = "root-control-plane")]
pub mod component_registry;
#[cfg(feature = "root-control-plane")]
pub mod deployment;
#[cfg(feature = "fleet-coordinator-canister")]
pub mod fleet_coordinator;
#[cfg(feature = "root-control-plane")]
pub mod fleet_registry_mirror;
#[cfg(feature = "root-control-plane")]
pub mod fleet_subnet_root;
#[cfg(feature = "root-control-plane")]
mod root_authority;
#[cfg(feature = "root-control-plane")]
pub mod runtime;
#[cfg(feature = "root-control-plane")]
pub mod state;
