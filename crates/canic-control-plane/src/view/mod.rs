//! Module: view
//!
//! Responsibility: expose internal read-only control-plane projections.
//! Does not own: persisted records, endpoint DTOs, or workflow decisions.
//! Boundary: receives values projected by ops before workflow use.

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
pub mod state;
