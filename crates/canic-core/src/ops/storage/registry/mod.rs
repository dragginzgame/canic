//! Module: ops::storage::registry
//!
//! Responsibility: expose deterministic canister registry storage operations.
//! Does not own: stable registry schemas, workflow orchestration, or endpoint DTOs.
//! Boundary: storage ops between topology workflows and stable registry records.

pub mod app;
pub mod mapper;
pub mod subnet;
