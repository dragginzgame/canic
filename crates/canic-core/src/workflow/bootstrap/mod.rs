//! Module: workflow::bootstrap
//!
//! Responsibility: group async bootstrap orchestration after lifecycle restore.
//! Does not own: IC lifecycle hooks, environment seeding, or stable-memory restore.
//! Boundary: lifecycle adapters schedule bootstrap work after runtime invariants hold.

pub mod nonroot;
