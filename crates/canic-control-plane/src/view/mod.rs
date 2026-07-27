//! Module: view
//!
//! Responsibility: expose internal read-only control-plane projections.
//! Does not own: persisted records, endpoint DTOs, or workflow decisions.
//! Boundary: receives values projected by ops before workflow use.

pub mod component_registry;
pub mod fleet_registry_mirror;
pub mod state;
